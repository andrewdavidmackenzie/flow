use std::collections::HashMap;
use std::fmt;

use serde_derive::{Deserialize, Serialize};

use flowrlib::input::InputInitializer;

use crate::compiler::loader::Validate;
use crate::errors::*;
use crate::model::name::HasName;
use crate::model::name::Name;
use crate::model::process::Process;
use crate::model::process::Process::FlowProcess;
use crate::model::process::Process::FunctionProcess;
use crate::model::route::HasRoute;
use crate::model::route::Route;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct ProcessReference {
    #[serde(default = "Name::default")]
    pub alias: Name,
    pub source: String,
    #[serde(rename = "input")]
    pub initializations: Option<HashMap<String, InputInitializer>>,
    // Map of initializers of inputs for this reference
    #[serde(skip)]
    pub process: Process,
}

impl ProcessReference {
    /// if the ProcessRef does not specify an alias for the process to be loaded
    /// then set the alias to be the name of the loaded process
    pub fn set_alias(&mut self) {
        if self.alias.is_empty() {
            self.alias = match self.process {
                FlowProcess(ref mut flow) => flow.name().clone(),
                FunctionProcess(ref mut function) => function.name().clone()
            };
        }
    }
}

impl HasName for ProcessReference {
    fn name(&self) -> &Name { &self.alias }
    fn alias(&self) -> &Name { &self.alias }
}

impl HasRoute for ProcessReference {
    fn route(&self) -> &Route {
        match self.process {
            Process::FlowProcess(ref flow) => flow.route(),
            Process::FunctionProcess(ref function) => function.route()
        }
    }

    fn route_mut(&mut self) -> &mut Route {
        match self.process {
            Process::FlowProcess(ref mut flow) => flow.route_mut(),
            Process::FunctionProcess(ref mut function) => function.route_mut()
        }
    }
}

impl Validate for ProcessReference {
    fn validate(&self) -> Result<()> {
        self.alias.validate()
    }
}

impl fmt::Display for ProcessReference {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\t\t\t\talias: {}\n\t\t\t\t\tsource: {}\n\t\t\t\t\tURL: {}\n",
               self.alias, self.source, self.source)
    }
}

#[cfg(test)]
mod test {
    use serde_json::json;

    use flowrlib::input::InputInitializer::{Always, Once};

    use super::ProcessReference;

    #[test]
    fn deserialize_simple() {
        let input_str = "
        alias = 'other'
        source = 'other.toml'
        ";

        let _reference: ProcessReference = toml::from_str(input_str).unwrap();
    }

    #[test]
    fn deserialize_with_once_input_initialization() {
        let input_str = "
        alias = 'other'
        source = 'other.toml'
        input.input1 = {once = 1}
        ";

        let reference: ProcessReference = toml::from_str(input_str).unwrap();
        let initialized_inputs = reference.clone().initializations.unwrap();
        assert_eq!(initialized_inputs.len(), 1, "Incorrect number of Input initializations parsed");
        match initialized_inputs.get("input1").unwrap() {
            Always(_) => panic!("Should have been a Once initializer"),
            Once(value) => assert_eq!(&json!(1), value, "input1 should be initialized to 1")
        }
   }

    /*
        For completeness I test the alternative format of expressing the table, but I prefer to use
        and will document the inline table that is tested below.
    */
    #[test]
    fn deserialize_with_constant_input_initialization() {
        let input_str = "
        alias = 'other'
        source = 'other.toml'
        input.input1 = {always = 1}
        ";

        let reference: ProcessReference = toml::from_str(input_str).unwrap();
        let initialized_inputs = reference.initializations.unwrap();
        assert_eq!(initialized_inputs.len(), 1, "Incorrect number of Input initializations parsed");
        match initialized_inputs.get("input1").unwrap() {
            Always(value) => {
                assert_eq!(&json!(1), value, "input1 should be initialized to 1");
            },
            Once(value) => {
                println!("initial_value: {}", value);
                panic!("Should have been a Constant initializer")
            }
        }
    }

    #[test]
    fn deserialize_with_constant_input_initialization_inline_table() {
        let input_str = "
        alias = 'other'
        source = 'other.toml'
        input.input1 = { always = 1 }
        ";

        let reference: ProcessReference = toml::from_str(input_str).unwrap();
        let initialized_inputs = reference.initializations.unwrap();
        assert_eq!(initialized_inputs.len(), 1, "Incorrect number of Input initializations parsed");
        match initialized_inputs.get("input1").unwrap() {
            Always(value) => {
                assert_eq!(&json!(1), value, "input1 should be initialized to 1");
            }
            Once(_) => panic!("Should have been an Always initializer"),
        }
    }

    #[test]
    fn deserialize_with_multiple_input_initialization() {
        let input_str = "
        alias = 'other'
        source = 'other.toml'
        input.input1 = {once = 1}
        input.input2 = {once = 'hello'}
        ";

        let reference: ProcessReference = toml::from_str(input_str).unwrap();
        let initialized_inputs = reference.initializations.unwrap();
        assert_eq!(initialized_inputs.len(), 2, "Incorrect number of Input initializations parsed");
        match initialized_inputs.get("input1").unwrap() {
            Once(value) => assert_eq!(&json!(1), value, "input1 should be initialized to 1"),
            _ => panic!("Should have been a Once initializer")
        }

        match initialized_inputs.get("input2").unwrap() {
            Once(value) => assert_eq!("hello", value, "input2 should be initialized to 'hello'"),
            _ => panic!("Should have been a Once initializer")
        }
    }

    #[test]
    fn deserialize_extra_field_fails() {
        let input_str = "
        alias = 'other'
        source = 'other.toml'
        foo = 'extra token'
        ";

        let reference: Result<ProcessReference, _> = toml::from_str(input_str);
        assert!(reference.is_err());
    }
}
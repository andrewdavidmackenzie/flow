use std::collections::HashMap;
use std::fmt;

use serde_derive::{Deserialize, Serialize};

use crate::errors::*;
use crate::model::input::InputInitializer;
use crate::model::name::HasName;
use crate::model::name::Name;
use crate::model::validation::Validate;

/// A `ProcessReference` is the struct used in a `Flow` to refer to a sub-process (Function or nested
/// Flow) it contains
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct ProcessReference {
    /// A reference may have an alias - this is used when multiple instances of the same Process
    /// are referenced from within a flow - they need difference aliases to distinguish between them
    /// in connections to/from them
    #[serde(default = "Name::default")]
    pub alias: Name,
    /// Relative or absolute source of the referenced process
    pub source: String,
    /// When a process is references, each reference can set different initial values on the inputs
    /// of the referenced process.
    #[serde(default, rename = "input")]
    pub initializations: HashMap<String, InputInitializer>,
}

impl ProcessReference {
    /// if the ProcessRef does not specify an alias for the process to be loaded
    /// then set the alias to be the name of the loaded process
    pub fn set_alias(&mut self, alias: &Name) {
        if self.alias.is_empty() {
            self.alias = alias.to_owned();
        }
    }
}

impl HasName for ProcessReference {
    fn name(&self) -> &Name {
        &self.alias
    }
    fn alias(&self) -> &Name {
        &self.alias
    }
}

impl Validate for ProcessReference {
    fn validate(&self) -> Result<()> {
        self.alias.validate()
    }
}

impl fmt::Display for ProcessReference {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "\t\t\t\talias: {}\n\t\t\t\t\tsource: {}\n\t\t\t\t\tURL: {}\n",
            self.alias, self.source, self.source
        )
    }
}

#[cfg(test)]
mod test {
    use serde_json::json;
    use url::Url;

    use crate::deserializers::deserializer::get_deserializer;
    use crate::errors::*;
    use crate::model::input::InputInitializer::{Always, Once};

    use super::ProcessReference;

    fn toml_from_str(content: &str) -> Result<ProcessReference> {
        let url = Url::parse("file:///fake.toml").expect("Could not parse URL");
        let deserializer =
            get_deserializer::<ProcessReference>(&url).expect("Could not get deserializer");
        deserializer.deserialize(content, Some(&url))
    }

    #[test]
    fn deserialize_simple() {
        let input_str = "
        alias = 'other'
        source = 'other.toml'
        ";

        let _reference: ProcessReference =
            toml_from_str(input_str).expect("Could not deserialize ProcessReference from toml");
    }

    #[test]
    fn deserialize_with_once_input_initialization() {
        let input_str = "
        alias = 'other'
        source = 'other.toml'
        input.input1 = {once = 1}
        ";

        let reference: ProcessReference =
            toml_from_str(input_str).expect("Could not deserialize ProcessReference from toml");
        assert_eq!(
            reference.initializations.len(),
            1,
            "Incorrect number of Input initializations parsed"
        );
        match reference.initializations.get("input1").unwrap() {
            Always(_) => panic!("Should have been a Once initializer"),
            Once(value) => assert_eq!(&json!(1), value, "input1 should be initialized to 1"),
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

        let reference: ProcessReference =
            toml_from_str(input_str).expect("Could not deserialize ProcessReference from toml");
        assert_eq!(
            reference.initializations.len(),
            1,
            "Incorrect number of Input initializations parsed"
        );
        match reference.initializations.get("input1") {
            Some(Always(value)) => {
                assert_eq!(&json!(1), value, "input1 should be initialized to 1")
            }
            _ => panic!("Should have been a Constant initializer"),
        }
    }

    #[test]
    fn deserialize_with_constant_input_initialization_inline_table() {
        let input_str = "
        alias = 'other'
        source = 'other.toml'
        input.input1 = { always = 1 }
        ";

        let reference: ProcessReference =
            toml_from_str(input_str).expect("Could not deserialize ProcessReference from toml");
        assert_eq!(
            reference.initializations.len(),
            1,
            "Incorrect number of Input initializations parsed"
        );
        match reference.initializations.get("input1") {
            Some(Always(value)) => {
                assert_eq!(&json!(1), value, "input1 should be initialized to 1")
            }
            _ => panic!("Should have been an Always initializer"),
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

        let reference: ProcessReference =
            toml_from_str(input_str).expect("Could not deserialize ProcessReference from toml");
        assert_eq!(
            reference.initializations.len(),
            2,
            "Incorrect number of Input initializations parsed"
        );
        match reference.initializations.get("input1") {
            Some(Once(value)) => assert_eq!(&json!(1), value, "input1 should be initialized to 1"),
            _ => panic!("Should have been a Once initializer"),
        }

        match reference.initializations.get("input2") {
            Some(Once(value)) => {
                assert_eq!("hello", value, "input2 should be initialized to 'hello'")
            }
            _ => panic!("Should have been a Once initializer"),
        }
    }

    #[test]
    fn deserialize_extra_field_fails() {
        let input_str = "
        alias = 'other'
        source = 'other.toml'
        foo = 'extra token'
        ";

        let reference: Result<ProcessReference> = toml_from_str(input_str);
        assert!(reference.is_err());
    }
}

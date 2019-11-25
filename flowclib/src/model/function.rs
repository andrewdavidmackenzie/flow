use std::fmt;

use error_chain::bail;
use serde_derive::{Deserialize, Serialize};

use crate::compiler::loader::Validate;
use crate::errors::*;
use crate::model::io::{IO, IOType};
use crate::model::io::IOSet;
use crate::model::name::HasName;
use crate::model::name::Name;
use crate::model::route::HasRoute;
use crate::model::route::Route;
use crate::model::route::SetIORoutes;
use crate::model::route::SetRoute;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct Function {
    #[serde(rename = "function")]
    name: Name,
    #[serde(default = "Function::default_impure")]
    impure: bool,
    implementation: Option<String>,
    #[serde(rename = "input")]
    pub inputs: IOSet,
    #[serde(rename = "output")]
    outputs: IOSet,

    #[serde(skip_deserializing)]
    alias: Name,
    #[serde(skip_deserializing, default = "Function::default_source_url")]
    source_url: String,
    #[serde(skip_deserializing)]
    route: Route,
    #[serde(skip_deserializing)]
    lib_reference: Option<String>,

    #[serde(skip_deserializing)]
    output_routes: Vec<(String, usize, usize)>,
    #[serde(skip_deserializing)]
    id: usize,
}

impl HasName for Function {
    fn name(&self) -> &Name { &self.name }
    fn alias(&self) -> &Name { &self.alias }
}

impl HasRoute for Function {
    fn route(&self) -> &Route {
        &self.route
    }
}

impl Function {
    pub fn set_id(&mut self, id: usize) {
        self.id = id;
    }

    pub fn get_id(&self) -> usize {
        self.id
    }

    pub fn is_impure(&self) -> bool {
        self.impure
    }

    pub fn get_inputs(&self) -> &IOSet {
        &self.inputs
    }

    pub fn get_mut_inputs(&mut self) -> &mut IOSet {
        &mut self.inputs
    }

    pub fn get_outputs(&self) -> IOSet {
        self.outputs.clone()
    }

    pub fn add_output_route(&mut self, output_route: (String, usize, usize)) {
        self.output_routes.push(output_route);
    }

    pub fn get_output_routes(&self) -> &Vec<(String, usize, usize)> {
        &self.output_routes
    }

    pub fn get_implementation(&self) -> Option<String> {
        self.implementation.clone()
    }

    pub fn set_implementation(&mut self, implementation: &str) {
        self.implementation = Some(implementation.to_owned());
    }

    pub fn get_source_url(&self) -> String {
        self.source_url.clone()
    }
}

impl Validate for Function {
    fn validate(&self) -> Result<()> {
        self.name.validate()?;

        let mut io_count = 0;

        if let Some(ref inputs) = self.inputs {
            for i in inputs {
                io_count += 1;
                i.validate()?
            }
        }

        if let Some(ref outputs) = self.outputs {
            for i in outputs {
                io_count += 1;
                i.validate()?
            }
        }

        // A function must have at least one valid input or output
        if io_count == 0 {
            bail!("A function must have at least one input or output");
        }

        Ok(())
    }
}

impl fmt::Display for Function {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "name: \t\t{}\n", self.name)?;
        write!(f, "alias: \t\t{}\n", self.alias)?;
        write!(f, "id: \t\t{}\n", self.id)?;

        write!(f, "inputs:\n")?;
        if let Some(ref inputs) = self.inputs {
            for input in inputs {
                write!(f, "\t{:#?}\n", input)?;
            }
        }

        write!(f, "outputs:\n")?;
        if let Some(ref outputs) = self.outputs {
            for output in outputs {
                write!(f, "\t{:#?}\n", output)?;
            }
        }

        Ok(())
    }
}

impl Default for Function {
    fn default() -> Function {
        Function {
            name: Name::default(),
            impure: false,
            implementation: None,
            alias: Name::default(),
            inputs: None,
            outputs: Some(vec!(IO::new("Json", &Route::default()))),
            source_url: Function::default_source_url(),
            route: Route::default(),
            lib_reference: None,
            output_routes: vec!(("".to_string(), 0, 0)),
            id: 0,
        }
    }
}

impl SetRoute for Function {
    fn set_routes_from_parent(&mut self, parent_route: &Route) {
        self.route = Route::from(format!("{}/{}", parent_route, self.alias));
        self.inputs.set_io_routes_from_parent(&self.route, IOType::FunctionIO);
        self.outputs.set_io_routes_from_parent(&self.route, IOType::FunctionIO);
    }
}

impl Function {
    fn default_source_url() -> String {
        "file:///".to_string()
    }

    fn default_impure() -> bool {
        false
    }

    pub fn set_alias(&mut self, alias: &Name) {
        self.alias = alias.clone();
    }

    pub fn set_implementation_url(&mut self, source: &str) {
        self.source_url = source.to_owned();
    }

    pub fn set_lib_reference(&mut self, lib_reference: Option<String>) {
        self.lib_reference = lib_reference
    }

    pub fn get_lib_reference(&self) -> &Option<String> {
        &self.lib_reference
    }
}

#[cfg(test)]
mod test {
    use toml;

    use crate::compiler::loader::Validate;
    use crate::model::datatype::DataType;
    use crate::model::io::Find;
    use crate::model::io::IOSet;
    use crate::model::name::HasName;
    use crate::model::name::Name;
    use crate::model::route::HasRoute;
    use crate::model::route::Route;
    use crate::model::route::SetRoute;

    use super::Function;

    impl Function {
        pub fn new(name: Name, impure: bool, implementation: Option<String>, alias: Name, inputs: IOSet, outputs: IOSet, source_url: &str,
                   route: Route, lib_reference: Option<String>, output_connections: Vec<(String, usize, usize)>,
                   id: usize) -> Self {
            Function {
                name,
                impure,
                implementation,
                alias,
                inputs,
                outputs,
                source_url: source_url.to_string(),
                route,
                lib_reference,
                output_routes: output_connections,
                id,
            }
        }
    }

    #[test]
    fn function_with_no_io_not_valid() {
        let fun = Function {
            name: Name::from("test_function"),
            impure: false,
            implementation: None,
            alias: Name::from("test_function"),
            source_url: Function::default_source_url(),
            inputs: Some(vec!()), // No inputs!
            outputs: None,         // No output!
            route: Route::default(),
            lib_reference: None,
            output_routes: vec!(("test_function".to_string(), 0, 0)),
            id: 0,
        };

        assert_eq!(fun.validate().is_err(), true);
    }

    #[test]
    #[should_panic]
    fn deserialize_missing_name() {
        let function_str = "
        type = 'Json'
        ";

        let _function: Function = toml::from_str(function_str).unwrap();
    }

    #[test]
    #[should_panic]
    fn deserialize_invalid() {
        let function_str = "
        name = 'test_function'
        ";

        let function: Function = toml::from_str(function_str).unwrap();
        function.validate().unwrap();
    }

    #[test]
    fn deserialize_output_empty() {
        let function_str = "
        function = 'test_function'
        [[output]]
        ";

        let function: Function = toml::from_str(function_str).unwrap();
        function.validate().unwrap();
        assert!(function.outputs.is_some());
    }

    #[test]
    #[should_panic]
    fn deserialize_extra_field_fails() {
        let function_str = "
        function = 'test_function'
        [[output]]
        foo = 'true'
        ";

        let function: Function = toml::from_str(function_str).unwrap();
        function.validate().unwrap();
    }

    #[test]
    fn deserialize_default_output() {
        let function_str = "
        function = 'test_function'
        [[output]]
        type = 'String'
        ";

        let function: Function = toml::from_str(function_str).unwrap();
        function.validate().unwrap();
        assert!(function.outputs.is_some());
        let output = &function.outputs.unwrap()[0];
        assert_eq!(*output.name(), Name::default());
        assert_eq!(output.datatype(0), DataType::from("String"));
    }

    #[test]
    fn deserialize_output_specified() {
        let function_str = "
        function = 'test_function'
        [[output]]
        name = 'sub_output'
        type = 'String'
        ";

        let function: Function = toml::from_str(function_str).unwrap();
        function.validate().unwrap();
        assert!(function.outputs.is_some());
        let output = &function.outputs.unwrap()[0];
        assert_eq!(*output.name(), Name::from("sub_output"));
        assert_eq!(output.datatype(0), DataType::from("String"));
    }

    #[test]
    fn deserialize_two_outputs_specified() {
        let function_str = "
        function = 'test_function'
        [[output]]
        name = 'sub_output'
        type = 'String'
        [[output]]
        name = 'other_output'
        type = 'Number'
        ";

        let function: Function = toml::from_str(function_str).unwrap();
        function.validate().unwrap();
        assert!(function.outputs.is_some());
        let outputs = function.outputs.unwrap();
        assert_eq!(outputs.len(), 2);
        let output0 = &outputs[0];
        assert_eq!(*output0.name(), Name::from("sub_output"));
        assert_eq!(output0.datatype(0), DataType::from("String"));
        let output1 = &outputs[1];
        assert_eq!(*output1.name(), Name::from("other_output"));
        assert_eq!(output1.datatype(0), DataType::from("Number"));
    }

    #[test]
    fn set_routes() {
        let function_str = "
        function = 'test_function'
        [[output]]
        name = 'sub_output'
        type = 'String'
        [[output]]
        name = 'other_output'
        type = 'Number'
        ";

        // Setup
        let mut function: Function = toml::from_str(function_str).unwrap();
        function.alias = Name::from("test_alias");

        // Test
        function.set_routes_from_parent(&Route::from("/flow"));

        assert_eq!(function.route, Route::from("/flow/test_alias"));

        let outputs = function.outputs.unwrap();

        let output0 = &outputs[0];
        assert_eq!(*output0.route(), Route::from("/flow/test_alias/sub_output"));

        let output1 = &outputs[1];
        assert_eq!(*output1.route(), Route::from("/flow/test_alias/other_output"));
    }

    #[test]
    fn get_array_element_of_root_output() {
        // Create a function where the output is an Array of String
        let function_str = "
        function = 'test_function'
        [[output]]
        type = 'Array/String'
        ";

        // Setup
        let mut function: Function = toml::from_str(function_str).unwrap();
        function.alias = Name::from("test_alias");
        function.set_routes_from_parent(&Route::from("/flow"));

        // Test
        // Try and get the output using a route to a specific element of the output
        let output = function.outputs.find_by_route(&Route::from("/0"), &None).unwrap();
        assert_eq!(*output.name(), Name::default());
    }
}
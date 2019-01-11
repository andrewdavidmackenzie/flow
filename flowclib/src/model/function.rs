use std::fmt;

use model::name::Name;
use model::name::HasName;
use model::io::IO;
use model::io::IOSet;
use model::route::Route;
use model::route::HasRoute;
use model::route::SetRoute;
use loader::loader::Validate;
use model::runnable::Runnable;
use serde_json::Value as JsonValue;
use url::Url;

#[derive(Deserialize, Debug, Clone)]
pub struct Function {
    #[serde(rename = "function")]
    name: Name,
    #[serde(rename = "input")]
    inputs: IOSet,
    #[serde(rename = "output")]
    outputs: IOSet,

    #[serde(skip_deserializing)]
    alias: Name,
    #[serde(skip_deserializing, default = "Function::default_url")]
    source_url: Url,
    #[serde(skip_deserializing)]
    route: Route,
    #[serde(skip_deserializing)]
    lib_reference: Option<String>,
    #[serde(skip_deserializing)]
    output_routes: Vec<(Route, usize, usize)>,
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

impl Runnable for Function {
    fn set_id(&mut self, id: usize) {
        self.id = id;
    }

    fn get_id(&self) -> usize {
        self.id
    }

    fn get_inputs(&self) -> IOSet {
        self.inputs.clone()
    }

    fn get_outputs(&self) -> IOSet {
        self.outputs.clone()
    }

    fn add_output_connection(&mut self, connection: (Route, usize, usize)) {
        self.output_routes.push(connection);
    }

    fn source_url(&self) -> Option<Url> {
        if self.lib_reference.is_none() {
            Some(self.source_url.clone())
        } else {
            None
        }
    }

    fn get_type(&self) -> &str {
        "Function"
    }

    fn is_static_value(&self) -> bool { false }

    fn get_output_routes(&self) -> &Vec<(Route, usize, usize)> {
        &self.output_routes
    }

    fn get_initial_value(&self) -> Option<JsonValue> { None }

    fn get_implementation(&self) -> &str {
        &self.name
    }
}

impl Validate for Function {
    fn validate(&self) -> Result<(), String> {
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
            return Err("A function must have at least one input or output".to_string());
        }

        Ok(())
    }
}

impl fmt::Display for Function {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\t\t\t\t\t\t\t\tname: \t\t{}\n", self.name)?;
        write!(f, "\t\t\t\t\t\t\t\talias: \t\t{}\n", self.alias)?;

        write!(f, "\t\t\t\t\t\t\t\tinputs:\n")?;
        if let Some(ref inputs) = self.inputs {
            for input in inputs {
                write!(f, "\t\t\t\t\t\t\t{:#?}\n", input)?;
            }
        }

        write!(f, "\t\t\t\t\t\t\t\toutput:\n")?;
        if let Some(ref outputs) = self.outputs {
            for output in outputs {
                write!(f, "\t\t\t\t\t\t\t{:#?}\n", output)?;
            }
        }

        Ok(())
    }
}

impl Default for Function {
    fn default() -> Function {
        Function {
            name: "".to_string(),
            alias: "".to_string(),
            inputs: None,
            outputs: Some(vec!(IO::new(&"Json".to_string(), &"".to_string()))),
            source_url: Function::default_url(),
            route: "".to_string(),
            lib_reference: None,
            id: 0,
            output_routes: vec!(("".to_string(), 0, 0)),
        }
    }
}

impl SetRoute for Function {
    fn set_routes_from_parent(&mut self, parent_route: &Route, flow_io: bool) {
        self.route = format!("{}/{}", parent_route, self.alias);
        self.inputs.set_routes_from_parent(&self.route, flow_io);
        self.outputs.set_routes_from_parent(&self.route, flow_io);
    }
}

impl Function {
    fn default_url() -> Url {
        Url::parse("file:///").unwrap()
    }

    pub fn new(name: Name, alias: Name, inputs: IOSet, outputs: IOSet, source_url: Url,
    route: Route, lib_reference: Option<String>, output_connections: Vec<(Route, usize, usize)>,
    id: usize) -> Self {
        Function {
            name, alias, inputs, outputs, source_url, route, lib_reference,
            output_routes: output_connections,  id
        }
    }

    pub fn set_alias(&mut self, alias: String) {
        self.alias = alias
    }

    pub fn set_source_url(&mut self, source: Url) {
        self.source_url = source
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
    use super::Function;
    use loader::loader::Validate;
    use toml;
    use model::name::HasName;
    use model::route::Route;
    use model::route::HasRoute;
    use model::route::SetRoute;
    use model::io::Find;

    #[test]
    fn function_with_no_io_not_valid() {
        let fun = Function {
            name: "test_function".to_string(),
            alias: "test_function".to_string(),
            source_url: Function::default_url(),
            inputs: Some(vec!()), // No inputs!
            outputs: None,         // No output!
            route: "".to_string(),
            lib_reference: None,
            id: 0,
            output_routes: vec!(("test_function".to_string(), 0, 0)),
        };

        assert_eq!(fun.validate().is_err(), true);
    }

    #[test]
    #[should_panic]
    fn deserialize_missing_name() {
        let function_str = "\
        type = \"Json\"
        ";

        let _function: Function = toml::from_str(function_str).unwrap();
    }

    #[test]
    #[should_panic]
    fn deserialize_invalid() {
        let function_str = "\
        name = \"test_function\"
        ";

        let function: Function = toml::from_str(function_str).unwrap();
        function.validate().unwrap();
    }

    #[test]
    fn deserialize_output_empty() {
        let function_str = "\
        function = \"test_function\"
        [[output]]
        ";

        let function: Function = toml::from_str(function_str).unwrap();
        function.validate().unwrap();
        assert!(function.outputs.is_some());
    }

    #[test]
    fn deserialize_default_output() {
        let function_str = "\
        function = \"test_function\"
        [[output]]
        type = \"String\"
        ";

        let function: Function = toml::from_str(function_str).unwrap();
        function.validate().unwrap();
        assert!(function.outputs.is_some());
        let output = &function.outputs.unwrap()[0];
        assert_eq!(output.name(), "");
        assert_eq!(output.datatype(0), "String");
    }

    #[test]
    fn deserialize_output_specified() {
        let function_str = "\
        function = \"test_function\"
        [[output]]
        name = \"sub_output\"
        type = \"String\"
        ";

        let function: Function = toml::from_str(function_str).unwrap();
        function.validate().unwrap();
        assert!(function.outputs.is_some());
        let output = &function.outputs.unwrap()[0];
        assert_eq!(output.name(), "sub_output");
        assert_eq!(output.datatype(0), "String");
    }

    #[test]
    fn deserialize_two_outputs_specified() {
        let function_str = "\
        function = \"test_function\"
        [[output]]
        name = \"sub_output\"
        type = \"String\"
        [[output]]
        name = \"other_output\"
        type = \"Number\"
        ";

        let function: Function = toml::from_str(function_str).unwrap();
        function.validate().unwrap();
        assert!(function.outputs.is_some());
        let outputs = function.outputs.unwrap();
        assert_eq!(outputs.len(), 2);
        let output0 = &outputs[0];
        assert_eq!(output0.name(), "sub_output");
        assert_eq!(output0.datatype(0), "String");
        let output1 = &outputs[1];
        assert_eq!(output1.name(), "other_output");
        assert_eq!(output1.datatype(0), "Number");
    }

    #[test]
    fn set_routes() {
        let function_str = "\
        function = \"test_function\"
        [[output]]
        name = \"sub_output\"
        type = \"String\"
        [[output]]
        name = \"other_output\"
        type = \"Number\"
        ";

        // Setup
        let mut function: Function = toml::from_str(function_str).unwrap();
        function.alias = "test_alias".to_string();

        // Test
        function.set_routes_from_parent(&Route::from("/flow"), false);

        assert_eq!(function.route, "/flow/test_alias");

        let outputs = function.outputs.unwrap();

        let output0 = &outputs[0];
        assert_eq!(output0.route(), "/flow/test_alias/sub_output");

        let output1 = &outputs[1];
        assert_eq!(output1.route(), "/flow/test_alias/other_output");
    }

    #[test]
    fn get_array_element_of_root_output() {
        // Create a function where the output is an Array of String
        let function_str = "\
        function = \"test_function\"
        [[output]]
        type = \"Array/String\"
        ";

        // Setup
        let mut function: Function = toml::from_str(function_str).unwrap();
        function.alias = "test_alias".to_string();
        function.set_routes_from_parent(&Route::from("/flow"), false);

        // Test
        // Try and get the output using a route to a specific element of the output
        let output = function.outputs.find_by_route(&Route::from("/0")).unwrap();
        assert_eq!(output.name(), "");
    }
}
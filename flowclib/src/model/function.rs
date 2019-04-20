use std::fmt;

use compiler::loader::Validate;
use flowrlib::function::Function as RuntimeFunction;
use flowrlib::url;
use model::io::{IO, IOType};
use model::io::IOSet;
use model::name::HasName;
use model::name::Name;
use model::route::HasRoute;
use model::route::Route;
use model::route::SetIORoutes;
use model::route::SetRoute;
use flowrlib::input::Input;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct Function {
    #[serde(rename = "function")]
    name: Name,
    #[serde(default = "Function::default_impure")]
    impure: bool,
    implementation: Option<String>,
    #[serde(rename = "input")]
    pub inputs: IOSet, // TODO
    #[serde(rename = "output")]
    outputs: IOSet,

    #[serde(skip_deserializing)]
    alias: Name,
    #[serde(skip_deserializing, default = "Function::default_url")]
    source_url: String,
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

    pub fn add_output_route(&mut self, output_route: (Route, usize, usize)) {
        self.output_routes.push(output_route);
    }

    pub fn get_output_routes(&self) -> &Vec<(Route, usize, usize)> {
        &self.output_routes
    }

    pub fn get_implementation_source(&self) -> String {
        if let Some(ref reference) = self.lib_reference {
            format!("lib://{}/{}", reference, &self.name)
        } else {
            match &self.implementation {
                Some(path) => {
                    url::join(&self.source_url, path)
                }
                None => {
                    let path = format!("No implementation found for provided function '{}'", self.name);
                    error!("{}", path);
                    path
                }
            }
        }
    }
}

impl From<Function> for RuntimeFunction {
    fn from(function: Function) -> Self {
        let mut process_inputs = vec!();
        match &function.get_inputs() {
            &None => {}
            Some(inputs) => {
                for input in inputs {
                    process_inputs.push(Input::from(input));
                }
            }
        };

        RuntimeFunction::new(function.alias().to_string(),
                             function.route().to_string(),
                             function.get_implementation_source(),
                             function.is_impure(),
                             process_inputs,
                             function.get_id(),
                             function.get_output_routes())
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
            name: "".to_string(),
            impure: false,
            implementation: None,
            alias: "".to_string(),
            inputs: None,
            outputs: Some(vec!(IO::new(&"Json".to_string(), &"".to_string()))),
            source_url: Function::default_url(),
            route: "".to_string(),
            lib_reference: None,
            output_routes: vec!(("".to_string(), 0, 0)),
            id: 0,
        }
    }
}

impl SetRoute for Function {
    fn set_routes_from_parent(&mut self, parent_route: &Route) {
        self.route = format!("{}/{}", parent_route, self.alias);
        self.inputs.set_io_routes_from_parent(&self.route, IOType::FunctionIO);
        self.outputs.set_io_routes_from_parent(&self.route, IOType::FunctionIO);
    }
}

impl Function {
    fn default_url() -> String {
        "file:///".to_string()
    }

    fn default_impure() -> bool {
        false
    }

    pub fn new(name: Name, impure: bool, implementation: Option<String>, alias: Name, inputs: IOSet, outputs: IOSet, source_url: String,
               route: Route, lib_reference: Option<String>, output_connections: Vec<(Route, usize, usize)>,
               id: usize) -> Self {
        Function {
            name,
            impure,
            implementation,
            alias,
            inputs,
            outputs,
            source_url,
            route,
            lib_reference,
            output_routes: output_connections,
            id,
        }
    }

    pub fn set_alias(&mut self, alias: String) {
        self.alias = alias
    }

    pub fn set_source_url(&mut self, source: &str) {
        self.source_url = source.to_string()
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
    use compiler::loader::Validate;
    use model::io::Find;
    use model::name::HasName;
    use model::route::HasRoute;
    use model::route::Route;
    use model::route::SetRoute;
    use toml;

    use super::Function;

    #[test]
    fn function_with_no_io_not_valid() {
        let fun = Function {
            name: "test_function".to_string(),
            impure: false,
            implementation: None,
            alias: "test_function".to_string(),
            source_url: Function::default_url(),
            inputs: Some(vec!()), // No inputs!
            outputs: None,         // No output!
            route: "".to_string(),
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
        assert_eq!(output.name(), "");
        assert_eq!(output.datatype(0), "String");
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
        assert_eq!(output.name(), "sub_output");
        assert_eq!(output.datatype(0), "String");
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
        assert_eq!(output0.name(), "sub_output");
        assert_eq!(output0.datatype(0), "String");
        let output1 = &outputs[1];
        assert_eq!(output1.name(), "other_output");
        assert_eq!(output1.datatype(0), "Number");
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
        function.alias = "test_alias".to_string();

        // Test
        function.set_routes_from_parent(&Route::from("/flow"));

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
        let function_str = "
        function = 'test_function'
        [[output]]
        type = 'Array/String'
        ";

        // Setup
        let mut function: Function = toml::from_str(function_str).unwrap();
        function.alias = "test_alias".to_string();
        function.set_routes_from_parent(&Route::from("/flow"));

        // Test
        // Try and get the output using a route to a specific element of the output
        let output = function.outputs.find_by_route(&Route::from("/0"), &None).unwrap();
        assert_eq!(output.name(), "");
    }
}
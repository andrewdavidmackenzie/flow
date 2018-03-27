use std::fmt;

use model::name::Name;
use model::name::HasName;
use model::datatype::DataType;
use model::datatype::HasDataType;
use model::connection::HasRoute;
use model::io::IO;
use model::io::IOSet;
use model::connection::Route;
use loader::loader::Validate;
use model::runnable::Runnable;
use serde_json::Value as JsonValue;
use url::Url;

#[derive(Deserialize, Debug, Clone)]
pub struct Function {
    pub name: Name,
    #[serde(rename = "input")]
    pub inputs: IOSet,
    #[serde(rename = "output")]
    pub outputs: IOSet,

    #[serde(skip_deserializing, default = "Function::default_url")]
    pub source_url: Url,
    #[serde(skip_deserializing)]
    pub route: Route,
    #[serde(skip_deserializing)]
    pub lib_reference: Option<String>,
    #[serde(skip_deserializing)]
    pub output_connections: Vec<(Route, usize, usize)>,
    #[serde(skip_deserializing)]
    pub id: usize,
}

impl HasName for Function {
    fn name(&self) -> &str {
        &self.name[..]
    }
}

impl Runnable for Function {
    fn set_id(&mut self, id: usize) {
        self.id = id;
    }

    fn get_id(&self) -> usize {
        self.id
    }

    fn get_inputs(&self) -> Option<Vec<IO>> {
        self.inputs.clone()
    }

    fn get_outputs(&self) -> Option<Vec<IO>> {
        self.outputs.clone()
    }

    fn add_output_connection(&mut self, connection: (Route, usize, usize)) {
        self.output_connections.push(connection);
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

    fn get_output_routes(&self) -> &Vec<(Route, usize, usize)> {
        &self.output_connections
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
        write!(f, "\t\t\t\t\t\t\t\tname: \t\t{}\n",
               self.name).unwrap();

        write!(f, "\t\t\t\t\t\t\t\tinputs:\n").unwrap();
        if let Some(ref inputs) = self.inputs {
            for input in inputs {
                write!(f, "\t\t\t\t\t\t\t{}\n", input).unwrap();
            }
        }

        write!(f, "\t\t\t\t\t\t\t\toutput:\n").unwrap();
        if let Some(ref outputs) = self.outputs {
            for output in outputs {
                write!(f, "\t\t\t\t\t\t\t{}\n", output).unwrap();
            }
        }

        Ok(())
    }
}

impl Default for Function {
    fn default() -> Function {
        Function {
            name: "".to_string(),
            inputs: None,
            outputs: Some(vec!(IO { name: "".to_string(), datatype: "Json".to_string(), route: "".to_string() })),
            source_url: Function::default_url(),
            route: "".to_string(),
            lib_reference: None,
            id: 0,
            output_connections: vec!(("".to_string(), 0, 0)),
        }
    }
}

impl Function {
    fn default_url() -> Url {
        Url::parse("file:///").unwrap()
    }

    pub fn set_routes(&mut self, parent_route: &str) {
        self.route = format!("{}/{}", parent_route, self.name);

        // Set routes to inputs
        if let Some(ref mut inputs) = self.inputs {
            for ref mut input in inputs {
                input.route = format!("{}/{}", self.route, input.name);
            }
        }

        // set routes to outputs
        if let Some(ref mut outputs) = self.outputs {
            for ref mut output in outputs {
                output.route = format!("{}/{}", self.route, output.name);
            }
        }
    }

    pub fn get<E: HasName + HasRoute + HasDataType>(&self,
                                                    collection: &Option<Vec<E>>,
                                                    name: &str)
                                                    -> Result<(Route, DataType, bool), String> {
        if let &Some(ref elements) = collection {
            for element in elements {
                if element.name() == name {
                    return Ok((format!("{}", element.route()), format!("{}", element.datatype()), false));
                }
            }
            return Err(format!("No IO with name '{}' was found", name));
        }
        Err(format!("No IO found."))
    }
}

#[cfg(test)]
mod test {
    use super::Function;
    use loader::loader::Validate;
    use toml;

    #[test]
    fn function_with_no_io_not_valid() {
        let fun = Function {
            name: "test_function".to_string(),
            source_url: Function::default_url(),
            inputs: Some(vec!()), // No inputs!
            outputs: None,         // No output!
            route: "".to_string(),
            lib_reference: None,
            id: 0,
            output_connections: vec!(("test_function".to_string(), 0, 0)),
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
        name = \"test_function\"
        [[output]]
        ";

        let function: Function = toml::from_str(function_str).unwrap();
        function.validate().unwrap();
        assert!(function.outputs.is_some());
    }

    #[test]
    fn deserialize_default_output() {
        let function_str = "\
        name = \"test_function\"
        [[output]]
        type = \"String\"
        ";

        let function: Function = toml::from_str(function_str).unwrap();
        function.validate().unwrap();
        assert!(function.outputs.is_some());
        let output = &function.outputs.unwrap()[0];
        assert_eq!(output.name, "");
        assert_eq!(output.datatype, "String");
    }

    #[test]
    fn deserialize_output_specified() {
        let function_str = "\
        name = \"test_function\"
        [[output]]
        name = \"sub_output\"
        type = \"String\"
        ";

        let function: Function = toml::from_str(function_str).unwrap();
        function.validate().unwrap();
        assert!(function.outputs.is_some());
        let output = &function.outputs.unwrap()[0];
        assert_eq!(output.name, "sub_output");
        assert_eq!(output.datatype, "String");
    }

    #[test]
    fn deserialize_two_outputs_specified() {
        let function_str = "\
        name = \"test_function\"
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
        assert_eq!(output0.name, "sub_output");
        assert_eq!(output0.datatype, "String");
        let output1 = &outputs[1];
        assert_eq!(output1.name, "other_output");
        assert_eq!(output1.datatype, "Number");
    }

    #[test]
    fn set_routes() {
        let function_str = "\
        name = \"test_function\"
        [[output]]
        name = \"sub_output\"
        type = \"String\"
        [[output]]
        name = \"other_output\"
        type = \"Number\"
        ";

        let mut function: Function = toml::from_str(function_str).unwrap();
        function.set_routes("/flow");

        assert_eq!(function.route, "/flow/test_function");

        let outputs = function.outputs.unwrap();

        let output0 = &outputs[0];
        assert_eq!(output0.route, "/flow/test_function/sub_output");

        let output1 = &outputs[1];
        assert_eq!(output1.route, "/flow/test_function/other_output");
    }
}
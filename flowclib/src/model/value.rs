use serde_json::Value as JsonValue;
use model::name::Name;
use model::name::HasName;
use model::connection::HasRoute;
use model::datatype::DataType;
use model::datatype::HasDataType;
use loader::loader::Validate;
use model::connection::Route;
use model::io::IO;
use model::io::IOSet;
use model::runnable::Runnable;
use url::Url;

use std::fmt;

#[derive(Deserialize, Clone)]
pub struct Value {
    pub name: Name,
    #[serde(rename = "type")]
    pub datatype: DataType,
    pub value: Option<JsonValue>,

    // Input to a value is assumed, at the route of the value itself and always possible
    // Output from a value is assumed, at the route of the value itself and always possible
    // Additional outputs that are parts of the default Output structure are possible at subpaths
    #[serde(rename = "output")]
    pub outputs: IOSet,

    // Input and Output routes are the same. We assume a value has an output as otherwise it's useless
    #[serde(skip_deserializing)]
    pub route: Route,
    #[serde(skip_deserializing)]
    pub output_connections: Vec<(Route, usize, usize)>,
    #[serde(skip_deserializing)]
    pub id: usize,
}

impl HasName for Value {
    fn name(&self) -> &str {
        &self.name[..]
    }
}

impl HasDataType for Value {
    fn datatype(&self) -> &str {
        &self.datatype[..]
    }
}

impl HasRoute for Value {
    fn route(&self) -> &str {
        &self.route[..]
    }
}

impl Runnable for Value {
    fn set_id(&mut self, id: usize) {
        self.id = id;
    }

    fn get_id(&self) -> usize {
        self.id
    }

    fn get_inputs(&self) -> IOSet {
        let value_input = IO {
            name: "".to_string(),
            datatype: self.datatype.clone(),
            route: self.route.clone(),
            flow_io: false
        };

        Some(vec!(value_input))
    }

    fn get_outputs(&self) -> IOSet {
        self.outputs.clone()
    }

    fn add_output_connection(&mut self, connection: (Route, usize, usize)) {
        self.output_connections.push(connection);
    }

    fn source_url(&self) -> Option<Url> {
        None
    }

    fn get_type(&self) -> &str {
        "Value"
    }

    fn get_output_routes(&self) -> &Vec<(Route, usize, usize)> {
        &self.output_connections
    }

    fn get_initial_value(&self) -> Option<JsonValue> {
        self.value.clone()
    }

    fn get_implementation(&self) -> &str {
        "Fifo"
    }
}

impl Validate for Value {
    fn validate(&self) -> Result<(), String> {
        self.datatype.validate()
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\tname: \t\t{}\n\t\t\t\t\troute: \t\t{}\n\t\t\t\t\tdatatype: \t{}\n",
               self.name, self.route, self.datatype).unwrap();
        if self.value.is_some() {
            write!(f, "\t\t\t\t\tvalue: \t\t{:?}", self.value).unwrap();
        }
        Ok(())
    }
}

impl Value {
    pub fn set_routes(&mut self, parent_route: &str) {
        // Set the route for this value
        self.route = format!("{}/{}", parent_route, self.name);

        // Specifying outputs in the spec is optional - so there could be none initially
        if self.outputs.is_none() {
            self.outputs = Some(vec!());
        }

        if let Some(ref mut outputs) = self.outputs {
            // Create an output for the "base"/"default" output of this value and insert at head of vec
            // of output routes
            let base_output = IO {
                name: "".to_string(),
                datatype: self.datatype.clone(),
                route: self.route.clone(),
                flow_io: false
            };
            outputs.insert(0, base_output);

            // Set sub routes for all outputs
            for ref mut output in outputs {
                if output.name.is_empty() {
                    output.route = self.route.clone();
                } else {
                    output.route = format!("{}/{}", self.route, output.name);
                }
            }
        }
    }

    pub fn get_input(&self) -> Result<IO, String> {
        Ok(IO {
            name: "".to_string(),
            route: self.route.clone(),
            datatype: self.datatype.clone(),
            flow_io: false
        })
    }

    pub fn get_output(&self, route: &str) -> Result<IO, String> {
        if let &Some(ref outputs) = &self.outputs {
            for output in outputs {
                if output.name == route {
                    return Ok(output.clone());
                }
            }
            return Err(format!("No output with name '{}' was found", route));
        }

        Err(format!("No output found."))
    }
}

#[cfg(test)]
mod test {
    use toml;
    use super::Value;
    use loader::loader::Validate;

    #[test]
    #[should_panic]
    fn deserialize_missing_name() {
        let value_str = "\
        type = \"Json\"
        ";

        let _value: Value = toml::from_str(value_str).unwrap();
    }

    #[test]
    #[should_panic]
    fn deserialize_missing_type() {
        let value_str = "\
        name = \"test_value\"
        ";

        let _value: Value = toml::from_str(value_str).unwrap();
    }

    #[test]
    fn deserialize_valid() {
        // No initial value, no outputs specified
        let value_str = "\
        name = \"test_value\"
        type = \"Json\"
        ";

        let value: Value = toml::from_str(value_str).unwrap();
        value.validate().unwrap();
        assert_eq!(value.name, "test_value");
        assert_eq!(value.datatype, "Json");
        assert!(value.value.is_none());
        assert!(value.outputs.is_none());
    }

    #[test]
    fn deserialize_initial_number_value() {
        // no outputs specified
        let value_str = "\
        name = \"test_value\"
        type = \"Json\"
        value = 10
        ";

        let value: Value = toml::from_str(value_str).unwrap();
        value.validate().unwrap();
        let initial_value = value.value.unwrap();
        assert_eq!(initial_value, json!(10));
    }

    #[test]
    fn deserialize_initial_string_value() {
        // no outputs specified
        let value_str = "\
        name = \"test_value\"
        type = \"Json\"
        value = \"Hello\"
        ";

        let value: Value = toml::from_str(value_str).unwrap();
        value.validate().unwrap();
        let initial_value = value.value.unwrap();
        assert_eq!(initial_value, json!("Hello"));
    }

    #[test]
    fn deserialize_output_empty() {
        let value_str = "\
        name = \"test_value\"
        type = \"Json\"
        value = \"Hello\"
        [[output]]
        ";

        let value: Value = toml::from_str(value_str).unwrap();
        value.validate().unwrap();
        assert!(value.outputs.is_some());
        let output = &value.outputs.unwrap()[0];
        assert_eq!(output.name, "");
        assert_eq!(output.datatype, "Json");
    }

    #[test]
    fn deserialize_output_specified() {
        let value_str = "\
        name = \"test_value\"
        type = \"Json\"
        value = \"Hello\"
        [[output]]
        name = \"sub_output\"
        type = \"String\"
        ";

        let value: Value = toml::from_str(value_str).unwrap();
        value.validate().unwrap();
        assert!(value.outputs.is_some());
        let output = &value.outputs.unwrap()[0];
        assert_eq!(output.name, "sub_output");
        assert_eq!(output.datatype, "String");
    }

    #[test]
    fn deserialize_two_outputs_specified() {
        let value_str = "\
        name = \"test_value\"
        type = \"Json\"
        value = \"Hello\"
        [[output]]
        name = \"sub_output\"
        type = \"String\"
        [[output]]
        name = \"other_output\"
        type = \"Number\"
        ";

        let value: Value = toml::from_str(value_str).unwrap();
        value.validate().unwrap();
        assert!(value.outputs.is_some());
        let outputs = value.outputs.unwrap();
        assert_eq!(outputs.len(), 2);
        let output0 = &outputs[0];
        assert_eq!(output0.name, "sub_output");
        assert_eq!(output0.datatype, "String");
        let output1 = &outputs[1];
        assert_eq!(output1.name, "other_output");
        assert_eq!(output1.datatype, "Number");
    }

    #[test]
    fn set_routes_base_route_only() {
        let value_str = "\
        name = \"test_value\"
        type = \"Json\"
        ";

        let mut value: Value = toml::from_str(value_str).unwrap();
        value.set_routes("/flow");

        assert_eq!(value.route, "/flow/test_value");

        let outputs = value.outputs.unwrap();
        assert_eq!(outputs.len(), 1);

        let base_output = &outputs[0];
        assert_eq!(base_output.route, "/flow/test_value");
    }

    #[test]
    fn set_routes_with_sub_routes() {
        let value_str = "\
        name = \"test_value\"
        type = \"Json\"
        value = \"Hello\"
        [[output]]
        name = \"sub_output\"
        type = \"String\"
        [[output]]
        name = \"other_output\"
        type = \"Number\"
        ";

        let mut value: Value = toml::from_str(value_str).unwrap();
        value.set_routes("/flow");

        assert_eq!(value.route, "/flow/test_value");

        let outputs = value.outputs.unwrap();

        let output0 = &outputs[0];
        assert_eq!(output0.route, "/flow/test_value");

        let output1 = &outputs[1];
        assert_eq!(output1.route, "/flow/test_value/sub_output");

        let output2 = &outputs[2];
        assert_eq!(output2.route, "/flow/test_value/other_output");
    }

    #[test]
    fn find_root_output() {
        let value_str = "\
        name = \"test_value\"
        type = \"Json\"
        ";

        let mut value: Value = toml::from_str(value_str).unwrap();
        value.set_routes("/flow");

        let output = value.get_output("").unwrap();
        assert_eq!(output.route, "/flow/test_value");
        assert_eq!(output.datatype, "Json");
        assert_eq!(output.flow_io, false);
    }


    #[test]
    fn find_named_output() {
        let value_str = "\
        name = \"test_value\"
        type = \"Json\"
        [[output]]
        name = \"sub_output\"
        type = \"String\"
        [[output]]
        name = \"other_output\"
        type = \"Number\"
        ";

        let mut value: Value = toml::from_str(value_str).unwrap();
        value.set_routes("/flow");

        let output = value.get_output("sub_output").unwrap();
        assert_eq!(output.route, "/flow/test_value/sub_output");
        assert_eq!(output.datatype, "String");
        assert_eq!(output.flow_io, false);
    }
}
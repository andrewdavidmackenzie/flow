use serde_json::Value as JsonValue;
use model::name::Name;
use model::name::HasName;
use model::datatype::DataType;
use model::datatype::HasDataType;
use loader::loader::Validate;
use model::route::Route;
use model::route::HasRoute;
use model::route::SetRoute;
use model::io::IO;
use model::io::IOSet;
use model::runnable::Runnable;
use url::Url;

use std::fmt;

#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Value {
    name: Name,
    #[serde(rename = "type")]
    datatype: DataType,
    init: Option<JsonValue>,
    #[serde(rename = "static", default = "default_static")]
    static_value: bool,

    // Input to a value is assumed, at the route of the value itself and always possible
    // Output from a value is assumed, at the route of the value itself and always possible
    // Additional outputs that are parts of the default Output structure are possible at subpaths
    #[serde(rename = "output")]
    outputs: IOSet,

    // Input and Output routes are the same. We assume a value has an output as otherwise it's useless
    #[serde(skip_deserializing)]
    route: Route,
    #[serde(skip_deserializing)]
    output_routes: Vec<(Route, usize, usize)>,
    #[serde(skip_deserializing)]
    id: usize,
}

fn default_static() -> bool {
    false
}

impl HasName for Value {
    fn name(&self) -> &Name { &self.name }
    fn alias(&self) -> &Name { &self.name }
}

impl HasDataType for Value {
    fn datatype(&self, level: usize) -> DataType {
        let type_levels: Vec<&str> = self.datatype.split('/').collect();
        DataType::from(type_levels[level])
    }
}

impl HasRoute for Value {
    fn route(&self) -> &Route {
        &self.route
    }
}

impl Runnable for Value {
    fn set_id(&mut self, id: usize) {
        self.id = id;
    }

    fn get_id(&self) -> usize {
        self.id
    }

    // TODO have this return a reference
    fn get_inputs(&self) -> IOSet {
        Some(vec!(IO::new(&self.datatype, &self.route)))
    }

    // TODO have this return a reference
    fn get_outputs(&self) -> IOSet {
        self.outputs.clone()
    }

    fn add_output_connection(&mut self, connection: (Route, usize, usize)) {
        self.output_routes.push(connection);
    }

    fn source_url(&self) -> Option<Url> {
        None
    }

    fn get_type(&self) -> &str {
        "Value"
    }

    fn is_static_value(&self) -> bool {
        self.static_value
    }

    fn get_output_routes(&self) -> &Vec<(Route, usize, usize)> {
        &self.output_routes
    }

    fn get_initial_value(&self) -> Option<JsonValue> {
        self.init.clone()
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
        write!(f, "name: \t\t{}\nid: \t\t{}\nroute: \t\t{}\ndatatype: \t{}\n",
               self.name, self.id, self.route, self.datatype).unwrap();
        if self.init.is_some() {
            write!(f, "initial value: \t\t{:?}", self.init).unwrap();
        }
        if self.static_value {
            write!(f, "static value: \t\t{:?}", true).unwrap();
        }
        Ok(())
    }
}

impl SetRoute for Value {
    fn set_routes_from_parent(&mut self, parent_route: &Route, flow_io: bool) {
        // Set the route for this value
        self.route = format!("{}/{}", parent_route, self.name);

        // Specifying outputs in the spec is optional - so there could be none initially
        if self.outputs.is_none() {
            self.outputs = Some(vec!());
        }

        if let Some(ref mut outputs) = self.outputs {
            // Create an output for the "base"/"default" output of this value and insert at head of vec
            // of output routes
            let base_output = IO::new(&self.datatype, &self.route);
            outputs.insert(0, base_output);
        }

        self.outputs.set_routes_from_parent(&self.route, flow_io);
    }
}

impl Value {
    pub fn new(name: Name,
               datatype: DataType,
               initial_value: Option<JsonValue>,
               static_value: bool,
               route: Route,
    outputs: IOSet, output_connections: Vec<(Route, usize, usize)>, id: usize) -> Self {
        Value {
            name, datatype,
            init: initial_value, static_value, route, outputs,
            output_routes: output_connections, id
        }
    }

    pub fn get_input(&self) -> Result<IO, String> {
        Ok(IO::new(&self.datatype, &self.route))
    }
}

#[cfg(test)]
mod test {
    use toml;
    use super::Value;
    use loader::loader::Validate;
    use model::name::HasName;
    use model::route::Route;
    use model::route::HasRoute;
    use model::route::SetRoute;
    use model::io::Find;
    use model::datatype::DataType;
    use model::name::Name;

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
        assert_eq!(value.name, Name::from("test_value"));
        assert_eq!(value.datatype, DataType::from("Json"));
        assert!(value.init.is_none());
        assert!(value.outputs.is_none());
    }

    #[test]
    #[should_panic]
    fn deserialize_extra_field_fails() {
        let value_str = "
        name = 'test_value'
        type = 'Json'
        extra = 'foo'
        ";

        let value: Value = toml::from_str(value_str).unwrap();
    }

    #[test]
    fn deserialize_initial_number_value() {
        // no outputs specified
        let value_str = "\
        name = \"test_value\"
        type = \"Json\"
        init = 10
        ";

        let value: Value = toml::from_str(value_str).unwrap();
        value.validate().unwrap();
        let initial_value = value.init.unwrap();
        assert_eq!(initial_value, json!(10));
    }

    #[test]
    fn initialized_static() {
        // no outputs specified
        let value_str = "\
        name = \"test_value\"
        type = \"Json\"
        init = 10
        static = true
        ";

        let value: Value = toml::from_str(value_str).unwrap();
        value.validate().unwrap();
    }

    #[test]
    fn deserialize_initial_string_value() {
        // no outputs specified
        let value_str = "\
        name = \"test_value\"
        type = \"Json\"
        init = \"Hello\"
        ";

        let value: Value = toml::from_str(value_str).unwrap();
        value.validate().unwrap();
        let initial_value = value.init.unwrap();
        assert_eq!(initial_value, json!("Hello"));
    }

    #[test]
    fn deserialize_output_empty() {
        let value_str = "\
        name = 'test_value'
        type = 'Json'
        init = 'Hello'
        [[output]]
        ";

        let value: Value = toml::from_str(value_str).unwrap();
        value.validate().unwrap();
        assert!(value.outputs.is_some());
        let output = &value.outputs.unwrap()[0];
        assert_eq!(output.name(), &Name::from(""));
        assert_eq!(output.datatype(0), DataType::from("Json"));
    }

    #[test]
    fn deserialize_output_specified() {
        let value_str = "\
        name = 'test_value'
        type = 'Json'
        init = 'Hello'
        [[output]]
        name = 'sub_output'
        type = 'String'
        ";

        let value: Value = toml::from_str(value_str).unwrap();
        value.validate().unwrap();
        assert!(value.outputs.is_some());
        let output = &value.outputs.unwrap()[0];
        assert_eq!(output.name(), &Name::from("sub_output"));
        assert_eq!(output.datatype(0), DataType::from("String"));
    }

    #[test]
    fn deserialize_two_outputs_specified() {
        let value_str = "\
        name = \"test_value\"
        type = \"Json\"
        init = \"Hello\"
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
        assert_eq!(output0.name(), &Name::from("sub_output"));
        assert_eq!(output0.datatype(0), DataType::from("String"));
        let output1 = &outputs[1];
        assert_eq!(output1.name(), &Name::from("other_output"));
        assert_eq!(output1.datatype(0), DataType::from("Number"));
    }

    #[test]
    fn set_routes_base_route_only() {
        let value_str = "\
        name = \"test_value\"
        type = \"Json\"
        ";

        let mut value: Value = toml::from_str(value_str).unwrap();
        value.set_routes_from_parent(&Route::from("/flow"), false);

        assert_eq!(value.route, Route::from("/flow/test_value"));

        let outputs = value.outputs.unwrap();
        assert_eq!(outputs.len(), 1);

        let base_output = &outputs[0];
        assert_eq!(base_output.route(), &Route::from("/flow/test_value"));
    }

    #[test]
    fn set_routes_with_sub_routes() {
        let value_str = "\
        name = \"test_value\"
        type = \"Json\"
        init = \"Hello\"
        [[output]]
        name = \"sub_output\"
        type = \"String\"
        [[output]]
        name = \"other_output\"
        type = \"Number\"
        ";

        let mut value: Value = toml::from_str(value_str).unwrap();
        value.set_routes_from_parent(&Route::from("/flow"), false);

        assert_eq!(value.route, Route::from("/flow/test_value"));

        let outputs = value.outputs.unwrap();

        let output0 = &outputs[0];
        assert_eq!(output0.route(), &Route::from("/flow/test_value"));

        let output1 = &outputs[1];
        assert_eq!(output1.route(), &Route::from("/flow/test_value/sub_output"));

        let output2 = &outputs[2];
        assert_eq!(output2.route(), &Route::from("/flow/test_value/other_output"));
    }

    #[test]
    fn find_root_output() {
        let value_str = "\
        name = \"test_value\"
        type = \"Json\"
        ";

        let mut value: Value = toml::from_str(value_str).unwrap();
        value.set_routes_from_parent(&Route::from("/flow"), false);

        let output = value.outputs.find_by_route(&Route::from("")).unwrap();
        assert_eq!(output.route(), &Route::from("/flow/test_value"));
        assert_eq!(output.datatype(0), DataType::from("Json"));
        assert_eq!(output.flow_io(), false);
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
        value.set_routes_from_parent(&Route::from("/flow"), false);

        let output = value.outputs.find_by_route(&Route::from("sub_output")).unwrap();
        assert_eq!(output.route(), &Route::from("/flow/test_value/sub_output"));
        assert_eq!(output.datatype(0), DataType::from("String"));
        assert_eq!(output.flow_io(), false);
    }
}
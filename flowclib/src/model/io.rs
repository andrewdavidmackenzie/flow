use model::name::Name;
use model::name::HasName;
use model::route::HasRoute;
use model::route::FindRoute;
use model::route::SetIORoutes;
use model::datatype::HasDataType;
use model::datatype::DataType;
use model::datatype::TypeCheck;
use compiler::loader::Validate;
use model::route::Route;
use std::collections::HashSet;
use model::route::Router;
use std::collections::HashMap;
use flowrlib::input::InputInitializer;

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub enum IOType {
    FunctionIO,
    FlowInput,
    FlowOutput
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct IO {
    #[serde(default = "default_name")]
    #[serde(skip_serializing_if = "String::is_empty")]
    name: Name,
    #[serde(rename = "type", default = "default_type")]
    datatype: DataType,
    #[serde(default = "default_depth")]
    depth: usize,

    #[serde(skip_deserializing)]
    route: Route,
    #[serde(skip_deserializing, default = "default_io_type")]
    io_type: IOType,
    #[serde(skip_deserializing)]
    initializer: Option<InputInitializer>,
}

impl Default for IO {
    fn default() -> Self {
        IO {
            name: default_name(),
            datatype: default_type(),
            depth: default_depth(),
            route: "".to_string(),
            io_type: IOType::FunctionIO,
            initializer: None,
        }
    }
}

impl HasName for IO {
    fn name(&self) -> &Name { &self.name }
    fn alias(&self) -> &Name { &self.name }
}

impl HasDataType for IO {
    fn datatype(&self, level: usize) -> DataType {
        self.datatype(level)
    }
}

impl IO {
    pub fn new(datatype: &DataType, route: &Route) -> Self {
        let mut io = IO::default();
        io.datatype = datatype.clone();
        io.route = route.clone();
        io
    }

    pub fn depth(&self) -> usize {
        self.depth
    }

    pub fn flow_io(&self) -> bool {
        self.io_type != IOType::FunctionIO
    }

    pub fn io_type(&self) -> &IOType {
        &self.io_type
    }

    pub fn set_flow_io(&mut self, io_type: IOType) {
        self.io_type = io_type;
    }

    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    pub fn datatype(&self, level: usize) -> DataType {
        let type_levels: Vec<&str> = self.datatype.split('/').collect();
        DataType::from(type_levels[level])
    }

    pub fn set_route(&mut self, route: &Route, io_type: &IOType) {
        self.route = route.clone();
        self.io_type = io_type.clone();
    }

    pub fn set_route_from_parent(&mut self, parent: &Route, io_type: &IOType) {
        let name = self.name().clone();

        if name.is_empty() {
            self.set_route(&parent, &io_type);
        } else {
            self.set_route(&format!("{}/{}", parent, name), &io_type);
        }
    }

    pub fn set_datatype(&mut self, datatype: &DataType) {
        self.datatype = datatype.clone()
    }

    pub fn get_initializer(&self) -> &Option<InputInitializer> {
        &self.initializer
    }

    pub fn set_initial_value(&mut self, initial_value: &Option<InputInitializer>) {
        // Avoid overwriting a possibly Some() value with a None value
        if initial_value.is_some() {
            self.initializer = initial_value.clone();
        }
    }
}

impl HasRoute for IO {
    fn route(&self) -> &Route {
        &self.route
    }
}

fn default_name() -> String {
    "".to_string()
}

fn default_type() -> String {
    "Json".to_string()
}

fn default_depth() -> usize {
    1
}

fn default_io_type() -> IOType { IOType::FunctionIO }

impl Validate for IO {
    fn validate(&self) -> Result<(), String> {
        self.datatype.valid()
    }
}

pub type IOSet = Option<Vec<IO>>;

impl Validate for IOSet {
    fn validate(&self) -> Result<(), String> {
        let mut name_set = HashSet::new();
        if let Some(ios) = self {
            for io in ios {
                io.validate()?;

                if io.name.is_empty() && ios.len() > 0 {
                    return Err("Cannot have empty IO name when there are multiple IOs".to_string());
                }

                if !name_set.insert(&io.name) {
                    return Err(format!("Two IOs cannot have the same name: '{}'", io.name));
                }
            }
        }
        Ok(())
    }
}

impl FindRoute for IOSet {
    /*
        Determine if it's a given route is in this IOSet
    */
    fn find(&self, route: &Route) -> bool {
        if let Some(ios) = self {
            for io in ios {
                if io.route() == route {
                    return true;
                }
            }
        }
        false
    }
}

impl SetIORoutes for IOSet {
    fn set_io_routes_from_parent(&mut self, parent: &Route, io_type: IOType) {
        if let &mut Some(ref mut ios) = self {
            for ref mut io in ios {
                io.set_route_from_parent(parent, &io_type)
            }
        }
    }
}

pub trait Find {
    fn find_by_name(&mut self, name: &Name, initial_value: &Option<InputInitializer>) -> Result<IO, String>;
    fn find_by_route(&mut self, route: &Route, initial_value: &Option<InputInitializer>) -> Result<IO, String>;
}

impl Find for IOSet {
    fn find_by_name(&mut self, name: &Name, initial_value: &Option<InputInitializer>) -> Result<IO, String> {
        if let Some(ref mut ios) = self {
            for mut io in ios {
                if io.name() == name {
                    io.set_initial_value(initial_value);
                    return Ok(io.clone());
                }
            }
            return Err(format!("No input or output with name '{}' was found", name));
        }
        Err(format!("No inputs or outputs found when looking for input/output named '{}'", name))
    }

    // TODO improve the Route handling of this - maybe moving into Router
    // TODO return a reference to the IO, with same lifetime as IOSet?
    fn find_by_route(&mut self, sub_route: &Route, initial_value: &Option<InputInitializer>) -> Result<IO, String> {
        if let Some(ref mut ios) = self {
            for mut io in ios {
                let (array_route, _num, array_index) = Router::without_trailing_array_index(sub_route);
                if array_index && (io.datatype(0).is_array()) && (io.name() == array_route.as_ref()) {
                    io.set_initial_value(initial_value);

                    let mut found = io.clone();
                    found.set_datatype(&io.datatype(1)); // the type within the array
                    let mut new_route = found.route().clone();
                    new_route.push_str(&format!("/{}", sub_route));
                    found.set_route(&new_route, &io.io_type);
                    return Ok(found);
                }

                if io.name() == sub_route {
                    io.set_initial_value(initial_value);
                    return Ok(io.clone());
                }
            }
            return Err(format!("No output with sub-route '{}' was found", sub_route));
        }

        Err(format!("No inputs or outputs found when looking for input/output with sub-route '{}'", sub_route))
    }
}

impl IO {
    pub fn set_initial_values(ios: &mut IOSet, initializers: &Option<HashMap<String, InputInitializer>>) {
        if let Some(inits) = initializers {
            if let Some(inputs) = ios {
                for initializer in inits {
                    // initializer.0 is io name, initializer.1 is the initial value to set it to
                    for (index, input) in inputs.iter_mut().enumerate() {
                        if input.name() == initializer.0.as_str() ||
                            (initializer.0.as_str() == "default" && index == 0) {
                            input.initializer = Some(initializer.1.clone());
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use toml;
    use super::IO;
    use compiler::loader::Validate;
    use model::name::HasName;
    use model::io::IOType;

    #[test]
    fn deserialize_empty_string() {
        let input_str = "";

        let output: IO = toml::from_str(input_str).unwrap();
        output.validate().unwrap();
        assert_eq!(output.datatype, "Json");
        assert_eq!(output.name, "");
    }

    #[test]
    fn deserialize_valid_type() {
        let input_str = "
        type = 'String'
        ";

        let output: IO = toml::from_str(input_str).unwrap();
        output.validate().unwrap();
    }

    #[test]
    #[should_panic]
    fn deserialize_invalid_type() {
        let input_str = "
        type = 'Unknown'
        ";

        let output: IO = toml::from_str(input_str).unwrap();
        output.validate().unwrap();
    }

    #[test]
    fn deserialize_name() {
        let input_str = "
        name = '/sub_route'
        type = 'String'
        ";

        let output: IO = toml::from_str(input_str).unwrap();
        output.validate().unwrap();
        assert_eq!(output.name, "/sub_route");
    }

    #[test]
    fn deserialize_valid_string_type() {
        let input_str = "
        name = 'input'
        type = 'String'
        ";

        let input: IO = toml::from_str(input_str).unwrap();
        input.validate().unwrap();
    }

    #[test]
    fn methods_work() {
        let input_str = "
        name = 'input'
        type = 'String'
        ";

        let input: IO = toml::from_str(input_str).unwrap();
        assert_eq!(input.name(), "input");
        assert_eq!(input.datatype(0), "String");
    }

    #[test]
    fn deserialize_valid_json_type() {
        let input_str = "
        name = 'input'
        type = 'Json'
        ";

        let input: IO = toml::from_str(input_str).unwrap();
        input.validate().unwrap();
    }

    #[test]
    #[should_panic]
    fn deserialize_extra_field_fails() {
        let input_str = "
        name = 'input'
        foo = 'extra token'
        type = 'Json'
        ";

        let input: IO = toml::from_str(input_str).unwrap();
        input.validate().unwrap();
    }

    #[test]
    fn unique_io_names_validate() {
        let io0 = IO {
            name: "io_name".to_string(),
            datatype: "String".to_string(),
            route: "".to_string(),
            depth: 1,
            io_type: IOType::FunctionIO,
            initializer: None,
        };
        let io1 = IO {
            name: "different_name".to_string(),
            datatype: "String".to_string(),
            route: "".to_string(),
            depth: 1,
            io_type: IOType::FunctionIO,
            initializer: None,
        };
        let ioset = Some(vec!(io0, io1));
        ioset.validate().unwrap()
    }

    #[test]
    #[should_panic]
    fn non_unique_io_names_wont_validate() {
        let io0 = IO {
            name: "io_name".to_string(),
            datatype: "String".to_string(),
            route: "".to_string(),
            depth: 1,
            io_type: IOType::FunctionIO,
            initializer: None,
        };
        let io1 = io0.clone();
        let ioset = Some(vec!(io0, io1));
        ioset.validate().unwrap()
    }

    #[test]
    #[should_panic]
    fn multiple_inputs_empty_name_not_allowed() {
        let io0 = IO {
            name: "io_name".to_string(),
            datatype: "String".to_string(),
            route: "".to_string(),
            depth: 1,
            io_type: IOType::FunctionIO,
            initializer: None,
        };
        let io1 = IO {
            name: "".to_string(),
            datatype: "String".to_string(),
            route: "".to_string(),
            depth: 1,
            io_type: IOType::FunctionIO,
            initializer: None,
        };
        let ioset = Some(vec!(io0, io1));
        ioset.validate().unwrap()
    }
}
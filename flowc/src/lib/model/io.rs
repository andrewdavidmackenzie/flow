use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt;

use error_chain::bail;
use serde_derive::{Deserialize, Serialize};

use flowcore::input::InputInitializer;

use crate::compiler::loader::Validate;
use crate::errors::*;
use crate::model::datatype::DataType;
use crate::model::datatype::HasDataType;
use crate::model::name::HasName;
use crate::model::name::Name;
use crate::model::route::HasRoute;
use crate::model::route::Route;
use crate::model::route::SetIORoutes;

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[allow(clippy::upper_case_acronyms)]
pub enum IOType {
    FunctionIO,
    FlowInput,
    FlowOutput,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
#[allow(clippy::upper_case_acronyms)]
pub struct IO {
    #[serde(default = "Name::default")]
    #[serde(skip_serializing_if = "Name::empty")]
    name: Name,
    #[serde(rename = "type", default = "default_type")]
    datatype: DataType,
    #[serde(rename = "value")]
    initializer: Option<InputInitializer>,

    #[serde(skip_deserializing)]
    route: Route,
    #[serde(skip_deserializing, default = "default_io_type")]
    io_type: IOType,
}

impl IO {
    pub fn new<D: Into<DataType>, R: Into<Route>>(datatype: D, route: R) -> Self {
        IO {
            datatype: datatype.into(),
            route: route.into(),
            ..Default::default()
        }
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

    pub fn datatype(&self) -> &DataType {
        &self.datatype
    }

    pub fn set_route(&mut self, route: &Route, io_type: &IOType) {
        self.route = route.clone();
        self.io_type = io_type.clone();
    }

    fn set_route_from_parent(&mut self, parent: &Route, io_type: &IOType) {
        if self.name().is_empty() {
            self.set_route(&parent, &io_type);
        } else {
            self.set_route(&Route::from(&format!("{}/{}", parent, self.name)), &io_type);
        }
    }

    pub fn set_datatype(&mut self, datatype: &DataType) {
        self.datatype = datatype.clone()
    }

    pub fn get_initializer(&self) -> &Option<InputInitializer> {
        &self.initializer
    }

    pub fn set_initializer(&mut self, initial_value: &Option<InputInitializer>) {
        // Avoid overwriting a possibly Some() value with a None value
        if initial_value.is_some() {
            self.initializer = initial_value.clone();
        }
    }
}

impl Default for IO {
    fn default() -> Self {
        IO {
            name: Name::default(),
            datatype: default_type(),
            route: Route::default(),
            io_type: IOType::FunctionIO,
            initializer: None,
        }
    }
}

impl fmt::Display for IO {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "'{}' @ '{}'", self.name, self.route)
    }
}

impl HasName for IO {
    fn name(&self) -> &Name {
        &self.name
    }
    fn alias(&self) -> &Name {
        &self.name
    }
}

impl HasDataType for IO {
    fn datatype(&self) -> &DataType {
        &self.datatype
    }
}

impl HasRoute for IO {
    fn route(&self) -> &Route {
        &self.route
    }

    fn route_mut(&mut self) -> &mut Route {
        &mut self.route
    }
}

fn default_type() -> DataType {
    DataType::from("Value")
}

fn default_io_type() -> IOType {
    IOType::FunctionIO
}

impl Validate for IO {
    fn validate(&self) -> Result<()> {
        self.datatype.valid()
    }
}

#[allow(clippy::upper_case_acronyms)]
pub type IOSet = Vec<IO>;

impl Validate for IOSet {
    fn validate(&self) -> Result<()> {
        let mut name_set = HashSet::new();
        for io in self {
            io.validate()?;

            if io.name.is_empty() && self.len() > 1 {
                bail!("Cannot have empty IO name when there are multiple IOs");
            }

            if !name_set.insert(&io.name) {
                bail!("Two IOs cannot have the same name: '{}'", io.name);
            }
        }

        Ok(())
    }
}

impl SetIORoutes for IOSet {
    fn set_io_routes_from_parent(&mut self, parent: &Route, io_type: IOType) {
        for io in self {
            io.set_route_from_parent(parent, &io_type)
        }
    }
}

pub trait Find {
    fn find(&self, route: &Route) -> bool;
    fn find_by_name(&mut self, name: &Name, initial_value: &Option<InputInitializer>)
        -> Result<IO>;
    fn find_by_route(
        &mut self,
        route: &Route,
        initial_value: &Option<InputInitializer>,
    ) -> Result<IO>;
}

impl Find for IOSet {
    fn find(&self, route: &Route) -> bool {
        for io in self {
            if io.route() == route {
                return true;
            }
        }
        false
    }

    fn find_by_name(
        &mut self,
        name: &Name,
        initial_value: &Option<InputInitializer>,
    ) -> Result<IO> {
        for io in self {
            if io.name() == name {
                io.set_initializer(initial_value);
                return Ok(io.clone());
            }
        }
        bail!("No input or output with name '{}' was found", name)
    }

    // TODO improve the Route handling of this - maybe moving into Router
    // TODO return a reference to the IO, with same lifetime as IOSet?
    fn find_by_route(
        &mut self,
        sub_route: &Route,
        initial_value: &Option<InputInitializer>,
    ) -> Result<IO> {
        for io in self {
            let (array_route, index, array_index) = sub_route.without_trailing_array_index();
            if array_index
                && (io.datatype().is_array())
                && (Route::from(io.name()) == array_route.into_owned())
            {
                io.set_initializer(initial_value);

                let mut found = io.clone();
                found.set_datatype(&io.datatype.within_array()?); // the type within the array
                let new_route = Route::from(format!("{}/{}", found.route(), index));
                found.set_route(&new_route, &io.io_type);
                return Ok(found);
            }

            if Route::from(io.name()) == *sub_route {
                io.set_initializer(initial_value);
                return Ok(io.clone());
            }
        }
        bail!("No output with sub-route '{}' was found", sub_route)
    }
}

impl IO {
    pub fn set_initial_values(ios: &mut IOSet, initializers: &HashMap<String, InputInitializer>) {
        for initializer in initializers {
            // initializer.0 is io name, initializer.1 is the initial value to set it to
            for (index, input) in ios.iter_mut().enumerate() {
                if *input.name() == Name::from(initializer.0)
                    || (initializer.0.as_str() == "default" && index == 0)
                {
                    input.initializer = Some(initializer.1.clone());
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::compiler::loader::Validate;
    use crate::model::datatype::DataType;
    use crate::model::io::{IOSet, IOType};
    use crate::model::name::HasName;
    use crate::model::name::Name;
    use crate::model::route::Route;

    use super::IO;

    #[test]
    fn deserialize_empty_string() {
        let output: IO = match toml::from_str("") {
            Ok(x) => x,
            Err(_) => panic!("TOML does not parse"),
        };
        assert!(output.validate().is_ok(), "IO does not validate()");
        assert_eq!(output.datatype, DataType::from("Value"));
        assert_eq!(output.name, Name::default());
    }

    #[test]
    fn deserialize_valid_type() {
        let input_str = "
        type = 'String'
        ";

        let output: IO = match toml::from_str(input_str) {
            Ok(x) => x,
            Err(_) => panic!("TOML does not parse"),
        };
        assert!(output.validate().is_ok(), "IO does not validate()");
    }

    #[test]
    fn deserialize_invalid_type() {
        let input_str = "
        type = 'Unknown'
        ";

        let output: IO = match toml::from_str(input_str) {
            Ok(x) => x,
            Err(_) => panic!("TOML does not parse"),
        };
        assert!(output.validate().is_err());
    }

    #[test]
    fn deserialize_name() {
        let input_str = "
        name = '/sub_route'
        type = 'String'
        ";

        let output: IO = match toml::from_str(input_str) {
            Ok(x) => x,
            Err(_) => panic!("TOML does not parse"),
        };
        assert!(output.validate().is_ok(), "IO does not validate()");
        assert_eq!("/sub_route", output.name.to_string());
    }

    #[test]
    fn deserialize_valid_string_type() {
        let input_str = "
        name = 'input'
        type = 'String'
        ";

        let input: IO = match toml::from_str(input_str) {
            Ok(x) => x,
            Err(_) => panic!("TOML does not parse"),
        };
        assert!(input.validate().is_ok(), "IO does not validate()");
    }

    #[test]
    fn methods_work() {
        let input_str = "
        name = 'input'
        type = 'String'
        ";

        let input: IO = match toml::from_str(input_str) {
            Ok(x) => x,
            Err(_) => panic!("TOML does not parse"),
        };
        assert_eq!(Name::from("input"), *input.name());
        assert_eq!(&DataType::from("String"), input.datatype());
    }

    #[test]
    fn deserialize_valid_json_type() {
        let input_str = "
        name = 'input'
        type = 'Value'
        ";

        let input: IO = match toml::from_str(input_str) {
            Ok(x) => x,
            Err(_) => panic!("TOML does not parse"),
        };
        assert!(input.validate().is_ok(), "IO does not validate()");
    }

    #[test]
    fn deserialize_extra_field_fails() {
        let input_str = "
        name = 'input'
        foo = 'extra token'
        type = 'Value'
        ";

        let input: Result<IO, _> = toml::from_str(input_str);
        assert!(input.is_err());
    }

    #[test]
    fn unique_io_names_validate() {
        let io0 = IO {
            name: Name::from("io_name"),
            datatype: DataType::from("String"),
            route: Route::default(),
            io_type: IOType::FunctionIO,
            initializer: None,
        };
        let io1 = IO {
            name: Name::from("different_name"),
            datatype: DataType::from("String"),
            route: Route::default(),
            io_type: IOType::FunctionIO,
            initializer: None,
        };
        let ioset = vec![io0, io1] as IOSet;
        assert!(ioset.validate().is_ok(), "IOSet does not validate()");
    }

    #[test]
    fn non_unique_io_names_wont_validate() {
        let io0 = IO {
            name: Name::from("io_name"),
            datatype: DataType::from("String"),
            route: Route::default(),
            io_type: IOType::FunctionIO,
            initializer: None,
        };
        let io1 = io0.clone();
        let ioset = vec![io0, io1] as IOSet;
        assert!(ioset.validate().is_err());
    }

    #[test]
    fn multiple_inputs_empty_name_not_allowed() {
        let io0 = IO {
            name: Name::from("io_name"),
            datatype: DataType::from("String"),
            route: Route::default(),
            io_type: IOType::FunctionIO,
            initializer: None,
        };
        let io1 = IO {
            name: Name::default(),
            datatype: DataType::from("String"),
            route: Route::default(),
            io_type: IOType::FunctionIO,
            initializer: None,
        };
        let ioset = vec![io0, io1] as IOSet;
        assert!(ioset.validate().is_err());
    }
}

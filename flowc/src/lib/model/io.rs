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

/// `IOType` defines what type of IO this is
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[allow(clippy::upper_case_acronyms)]
pub enum IOType {
    /// The IO is an input or an output of a Function
    FunctionIO,
    /// The IO is the input to a Flow
    FlowInput,
    /// The IO is the output of a Flow
    FlowOutput,
}

/// `IO` contains information about the Input or Output of a Function or Flow
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
#[allow(clippy::upper_case_acronyms)]
pub struct IO {
    /// An IO can have a specific name or if the only one be empty
    #[serde(default = "Name::default")]
    #[serde(skip_serializing_if = "Name::empty")]
    pub(crate) name: Name,
    /// WHat Datatype is accepted on this input or generated by this output
    #[serde(rename = "type", default = "default_type")]
    pub(crate) datatype: DataType,
    /// If an input, does it have an initializer that puts an initial value on the Input
    #[serde(rename = "value")]
    pub(crate) initializer: Option<InputInitializer>,

    /// `route` defines where in the flow hierarchy this IO is located
    #[serde(skip_deserializing)]
    pub(crate) route: Route,
    /// What type of IO is this, used in making connections between IOs
    #[serde(skip_deserializing, default = "default_io_type")]
    pub(crate) io_type: IOType,
}

impl IO {
    /// Create a new IO with a specific datatype and at a specific route
    pub fn new<D: Into<DataType>, R: Into<Route>>(datatype: D, route: R) -> Self {
        IO {
            datatype: datatype.into(),
            route: route.into(),
            ..Default::default()
        }
    }

    /// Is this IO an input or an output of a Flow?
    pub fn flow_io(&self) -> bool {
        self.io_type != IOType::FunctionIO
    }

    /// Return a reference to the IOType of this IO
    pub fn io_type(&self) -> &IOType {
        &self.io_type
    }

    /// Set the IO type
    pub fn set_io_type(&mut self, io_type: IOType) {
        self.io_type = io_type;
    }

    /// Return a reference to the data type this IO generates or accepts
    pub fn datatype(&self) -> &DataType {
        &self.datatype
    }

    /// Set the route where this IO resides in the flow hierarchy
    pub fn set_route(&mut self, route: &Route, io_type: &IOType) {
        self.route = route.clone();
        self.io_type = io_type.clone();
    }

    /// Set the route of this IO based on the route of the parent it is located within and it's name
    fn set_route_from_parent(&mut self, parent: &Route, io_type: &IOType) {
        if self.name().is_empty() {
            self.set_route(parent, io_type);
        } else {
            self.set_route(&Route::from(&format!("{}/{}", parent, self.name)), io_type);
        }
    }

    /// Set the datatype of this IO
    pub fn set_datatype(&mut self, datatype: &DataType) {
        self.datatype = datatype.clone()
    }

    /// Get a reference to the input initializer of this IO
    pub fn get_initializer(&self) -> &Option<InputInitializer> {
        &self.initializer
    }

    /// Set the input initializer of this IO
    pub fn set_initializer(&mut self, initial_value: &Option<InputInitializer>) {
        // Avoid overwriting a possibly Some() value with a None value
        if initial_value.is_some() && self.initializer.is_none() {
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

/// An `IOSet` is a set of IOs belonging to a function or a flow
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

/// `Find` trait is implemented by a number of object types to help find a sub-object
/// using it's Name or Route
pub trait Find {
    /// Find a sub-object using it's Route
    fn find(&self, route: &Route) -> bool;
    /// Find a sub-object (Input) using it's name and set the input initializer on it
    fn find_by_name_and_set_initializer(
        &mut self,
        name: &Name,
        initial_value: &Option<InputInitializer>,
    ) -> Result<IO>;

    /// Find a sub-object (Input) using it's Route and set the input initializer on it
    fn find_by_route_and_set_initializer(
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

    fn find_by_name_and_set_initializer(
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
    fn find_by_route_and_set_initializer(
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
                found.set_datatype(&io.datatype.within_array().ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::Other, "DataType is not an Array")
                })?); // the type within the array
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

#[cfg(test)]
mod test {
    use url::Url;

    use flowcore::deserializers::deserializer::get_deserializer;
    use flowcore::errors::*;

    use crate::compiler::loader::Validate;
    use crate::model::datatype::DataType;
    use crate::model::io::{IOSet, IOType};
    use crate::model::name::HasName;
    use crate::model::name::Name;
    use crate::model::route::Route;

    use super::IO;

    fn toml_from_str(content: &str) -> Result<IO> {
        let url = Url::parse("file:///fake.toml").expect("Could not parse URL");
        let deserializer = get_deserializer::<IO>(&url).expect("Could not get deserializer");
        deserializer.deserialize(content, Some(&url))
    }

    #[test]
    fn deserialize_empty_string() {
        let output: IO = match toml_from_str("") {
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

        let output: IO = match toml_from_str(input_str) {
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

        let output: IO = match toml_from_str(input_str) {
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

        let output: IO = match toml_from_str(input_str) {
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

        let input: IO = match toml_from_str(input_str) {
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

        let input: IO = match toml_from_str(input_str) {
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

        let input: IO = match toml_from_str(input_str) {
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

        let input: Result<IO> = toml_from_str(input_str);
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

use std::collections::HashSet;
use std::fmt;

use error_chain::bail;
use serde_derive::{Deserialize, Serialize};

use crate::errors::*;
use crate::model::datatype::{DataType, OBJECT_TYPE};
use crate::model::datatype::HasDataTypes;
use crate::model::input::InputInitializer;
use crate::model::name::HasName;
use crate::model::name::Name;
use crate::model::route::HasRoute;
use crate::model::route::Route;
use crate::model::route::SetIORoutes;
use crate::model::validation::Validate;

/// `IOType` defines what type of IO this is
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[allow(clippy::upper_case_acronyms)]
pub enum IOType {
    /// The IO is an input of a Function
    FunctionInput,
    /// The IO is an output of a Function
    FunctionOutput,
    /// The IO is the input to a Flow
    FlowInput,
    /// The IO is the output of a Flow
    FlowOutput,
}

impl Default for IOType {
    fn default() -> Self {
        IOType::FunctionInput
    }
}

/// `IO` contains information about the Input or Output of a `Function` or `Flow`
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
#[allow(clippy::upper_case_acronyms)]
pub struct IO {
    /// An IO can have a specific name or if the only one be empty
    #[serde(default = "Name::default")]
    #[serde(skip_serializing_if = "Name::empty")]
    name: Name,

    /// What Datatypes are accepted on this input or generated by this output
    #[serde(rename = "type", default = "default_types",
    deserialize_with = "super::datatype_array_serde::datatype_or_datatype_array")]
    datatypes: Vec<DataType>,
    
    /// If an input, does it have an initializer that puts an initial value on the Input
    #[serde(rename = "value")]
    initializer: Option<InputInitializer>,

    /// `route` defines where in the full flow hierarchy this IO is located, including it's `name`
    /// as the last segment
    #[serde(skip_deserializing)]
    route: Route,
    
    /// What type of IO is this, used in making connections between IOs
    #[serde(skip_deserializing, default = "IOType::default")]
    io_type: IOType,
}

impl IO {
    /// Create a new `IO` with a specific datatype and at a specific `Route`
    pub fn new<R: Into<Route>>(datatypes: Vec<DataType>, route: R) -> Self {
        IO {
            datatypes,
            route: route.into(),
            ..Default::default()
        }
    }

    /// Create a new `IO` with a specific datatype and at a specific `Route` and a `Name`
    pub fn new_named<R: Into<Route>, N: Into<Name>>(
        datatypes: Vec<DataType>,
        route: R,
        name: N,
    ) -> Self {
        IO {
            datatypes,
            route: route.into(),
            name: name.into(),
            ..Default::default()
        }
    }

    /// Is this IO an input or an output of a Flow?
    pub fn flow_io(&self) -> bool {
        self.io_type == IOType::FlowInput || self.io_type == IOType::FlowOutput
    }

    /// Is this IO an input or an output of a Function?
    pub fn function_io(&self) -> bool {
        self.io_type == IOType::FunctionInput || self.io_type == IOType::FunctionOutput
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
    pub fn datatypes(&self) -> &Vec<DataType> {
        &self.datatypes
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

    /// Set the datatypes of this IO
    pub fn set_datatypes(&mut self, datatypes: &[DataType]) {
        self.datatypes = datatypes.to_vec()
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

impl HasDataTypes for IO {
    fn datatypes(&self) -> &Vec<DataType> {
        &self.datatypes
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

fn default_types() -> Vec<DataType> {
    vec!(DataType::from(OBJECT_TYPE))
}

/*fn default_io_type() -> IOType {
    IOType::FunctionIO
}*/

impl Validate for IO {
    fn validate(&self) -> Result<()> {
        if self.datatypes.is_empty() {
            bail!("There must be one or more valid data types specified on an IO");
        }
        for datatype in self.datatypes() {
            datatype.valid()?;
        }
        Ok(())
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
    /// Find IO using it's sub-Route and set the input initializer on it
    fn find_by_subroute_and_set_initializer(
        &mut self,
        subroute: &Route,
        initial_value: &Option<InputInitializer>,
    ) -> Result<IO>;
}

impl Find for IOSet {
    // TODO improve the Route handling of this - maybe moving into Router
    // TODO return a reference to the IO, with same lifetime as IOSet?
    fn find_by_subroute_and_set_initializer(
        &mut self,
        sub_route: &Route,
        initial_value: &Option<InputInitializer>,
    ) -> Result<IO> {
        for io in self {
            for datatype in io.datatypes().clone() { // TODO remove need for clone
                let (array_route, index, array_index) = sub_route.without_trailing_array_index();
                if array_index
                    && (datatype.is_array())
                    && (Route::from(io.name()) == array_route.into_owned())
                {
                    io.set_initializer(initial_value);

                    let mut found = io.clone();

                    // Set the datatype of the found IO to be the type within the array of types
                    // and this will be converted by the runtime during execution
                    found.set_datatypes(&[datatype.within_array()?]);

                    let new_route = Route::from(format!("{}/{}", found.route(), index));
                    found.set_route(&new_route, &io.io_type);
                    return Ok(found);
                }

                if Route::from(io.name()) == *sub_route {
                    io.set_initializer(initial_value);
                    return Ok(io.clone());
                }
            }
        }
        bail!("No output with sub-route '{}' was found", sub_route)
    }
}

#[cfg(test)]
mod test {
    use url::Url;

    use crate::deserializers::deserializer::get_deserializer;
    use crate::errors::*;
    use crate::model::datatype::{DataType, OBJECT_TYPE, STRING_TYPE};
    use crate::model::io::{IOSet, IOType};
    use crate::model::name::HasName;
    use crate::model::name::Name;
    use crate::model::route::Route;
    use crate::model::validation::Validate;

    use super::Find;
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
        assert_eq!(output.datatypes[0], DataType::from(OBJECT_TYPE));
        assert_eq!(output.name, Name::default());
    }

    #[test]
    fn deserialize_valid_type() {
        let input_str = "
        type = 'string'
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
        type = 'string'
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
        type = 'string'
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
        type = 'string'
        ";

        let input: IO = match toml_from_str(input_str) {
            Ok(x) => x,
            Err(_) => panic!("TOML does not parse"),
        };
        assert_eq!(Name::from("input"), *input.name());
        assert_eq!(1, input.datatypes().len());
        assert_eq!(DataType::from(STRING_TYPE), input.datatypes()[0]);
    }

    #[test]
    fn deserialize_valid_json_type() {
        let input_str = "
        name = 'input'
        type = 'object'
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
        type = 'object'
        ";

        let input: Result<IO> = toml_from_str(input_str);
        assert!(input.is_err());
    }

    #[test]
    fn unique_io_names_validate() {
        let io0 = IO {
            name: Name::from("io_name"),
            datatypes: vec!(DataType::from(STRING_TYPE)),
            io_type: IOType::FunctionInput,
            initializer: None,
            ..Default::default()
        };
        let io1 = IO {
            name: Name::from("different_name"),
            datatypes: vec!(DataType::from(STRING_TYPE)),
            io_type: IOType::FunctionInput,
            initializer: None,
            ..Default::default()
        };
        let ioset = vec![io0, io1] as IOSet;
        assert!(ioset.validate().is_ok(), "IOSet does not validate()");
    }

    #[test]
    fn non_unique_io_names_wont_validate() {
        let io0 = IO {
            name: Name::from("io_name"),
            datatypes: vec!(DataType::from(STRING_TYPE)),
            io_type: IOType::FunctionInput,
            initializer: None,
            ..Default::default()
        };
        let io1 = io0.clone();
        let ioset = vec![io0, io1] as IOSet;
        assert!(ioset.validate().is_err());
    }

    #[test]
    fn multiple_inputs_empty_name_not_allowed() {
        let io0 = IO {
            name: Name::from("io_name"),
            datatypes: vec!(DataType::from(STRING_TYPE)),
            io_type: IOType::FunctionInput,
            initializer: None,
            ..Default::default()
        };
        let io1 = IO {
            datatypes: vec!(DataType::from(STRING_TYPE)),
            io_type: IOType::FunctionInput,
            initializer: None,
            ..Default::default()
        };
        let ioset = vec![io0, io1] as IOSet;
        assert!(ioset.validate().is_err());
    }

    #[test]
    fn no_datatypes_not_allowed() {
        let io = IO {
            name: Name::from("io_name"),
            datatypes: vec!(),
            io_type: IOType::FunctionInput,
            initializer: None,
            ..Default::default()
        };
        assert!(io.validate().is_err());
    }

    #[test]
    fn get_array_element_of_root_output() {
        let mut outputs = vec![IO::new(vec![DataType::from("array/integer")], "")] as IOSet;

        // Test
        // Try and get the output using a route to a specific element of the output
        let output = outputs
            .find_by_subroute_and_set_initializer(&Route::from("/0"), &None)
            .expect("Expected to find an IO");
        assert_eq!(*output.name(), Name::default());
    }
}

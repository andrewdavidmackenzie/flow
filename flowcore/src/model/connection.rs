use std::fmt;

use log::debug;
use serde_derive::{Deserialize, Serialize};

use crate::errors::*;
use crate::model::datatype::{DataType, OBJECT_TYPE};
use crate::model::io::IO;
use crate::model::name::Name;
use crate::model::route::HasRoute;
use crate::model::route::Route;
use crate::model::validation::Validate;

/// `Connection` defines a connection between the output of one function or flow to the input
/// of another function or flow and maybe optionally named for legibility/debugging.
#[derive(Deserialize, Serialize, Default, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct Connection {
    /// Optional name given to a connection for legibility and debugging
    #[serde(default, skip_serializing_if = "String::is_empty")]
    name: Name,
    /// `from` defines the origin of the connection
    from: Route,
    /// `to` defines the destination(s) of this connection
    #[serde(deserialize_with = "super::route_array_serde::route_or_route_array")]
    to: Vec<Route>,
    /// `from_io` is used during the compilation process and refers to a found output for the connection
    // TODO make these references, not clones
    #[serde(skip)]
    from_io: IO,
    /// `to_io` is used during the compilation process and refers to a found input for the connection
    #[serde(skip)]
    to_io: IO,
    /// `level` defines at what level in the flow hierarchy of nested flows this connections belongs
    #[serde(skip)]
    level: usize,
}

/// `Direction` defines whether a `Connection` is coming from an IO or to an IO
#[derive(Debug)]
#[allow(clippy::upper_case_acronyms)]
pub enum Direction {
    /// The `Connection` is `FROM` this IO to another
    FROM,
    /// The `Connection` is `TO` this IO from another
    TO,
}

impl fmt::Display for Connection {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match (self.from_io.flow_io(), self.to_io.flow_io()) {
            (true, true) => write!(
                f,
                "(f){} --> {}(f)",
                self.from_io.route(),
                self.to_io.route()
            ),
            (true, false) => write!(f, "(f){} --> {}", self.from_io.route(), self.to_io.route()),
            (false, true) => write!(
                f,
                "   {} --> {}(f)",
                self.from_io.route(),
                self.to_io.route()
            ),
            (false, false) => write!(f, "   {} --> {}", self.from_io.route(), self.to_io.route()),
        }
    }
}

impl Validate for Connection {
    // TODO maybe validate some of the combinations here that are checked in build_connections here?
    fn validate(&self) -> Result<()> {
        self.name.validate()?;
        self.from.validate()?;
        for destination in &self.to {
            destination.validate()?;
        }
        Ok(())
    }
}

impl Connection {
    /// Create a new Route with `from_route` as the source `Route` and `to_route` as the destination
    pub fn new<R>(from_route: R, to_route: R) -> Self
    where
        R: Into<Route>,
    {
        Connection {
            from: from_route.into(),
            to: vec![to_route.into()],
            ..Default::default()
        }
    }

    /// Return the name
    #[cfg(feature = "debugger")]
    pub fn name(&self) -> &Name {
        &self.name
    }

    /// Connect the `from_io` to the `to_io` if they are compatible
    pub fn connect(&mut self, from_io: IO, to_io: IO, level: usize) -> Result<()> {
        if Self::compatible_types(from_io.datatypes(), to_io.datatypes(), &self.from) {
            debug!(
                "Connection built from '{}' to '{}'",
                from_io.route(),
                to_io.route()
            );
            self.from_io = from_io;
            self.to_io = to_io;
            self.level = level;
            return Ok(());
        }

        bail!("Cannot connect types: from {:#?} to\n{:#?}", from_io, to_io)
    }

    /// Return the `from` Route specified in this connection
    pub fn from(&self) -> &Route {
        &self.from
    }

    /// Return a reference to the from_io
    pub fn from_io(&self) -> &IO {
        &self.from_io
    }

    /// Return the `to` Route specified in this connection
    pub fn to(&self) -> &Vec<Route> {
        &self.to
    }

    /// Return a mutable reference to the from_io
    pub fn from_io_mut(&mut self) -> &mut IO {
        &mut self.from_io
    }

    /// Return a reference to the to_io
    pub fn to_io(&self) -> &IO {
        &self.to_io
    }

    /// Return a mutable reference to the to_io
    pub fn to_io_mut(&mut self) -> &mut IO {
        &mut self.to_io
    }

    /// Get at what level in the flow hierarchy this connection exists (source)
    pub fn level(&self) -> usize {
        self.level
    }

    /// For a set of output types to be compatible with a destination's set of types
    /// ALL of the output_types must have a compatible input type, to guarantee that any
    /// of the valid types produced can be handled by the destination
    fn compatible_types(from: &[DataType], to: &[DataType], from_route: &Route) -> bool {
        if from.is_empty() || to.is_empty() {
            return false;
        }

        for output_type in from {
            let mut compatible_destination_type = false;
            for input_type in to {
                if Self::two_compatible_types(output_type, input_type, from_route) {
                    compatible_destination_type = true;
                }
            }
            if !compatible_destination_type {
                return false; // we could not find a compatible_destination_type for this output_type
            }
        }

        true // all output_types found a compatible destination type
    }

    /// Determine if the type of the source of a connection and the type of the destination are
    /// compatible, what type of conversion maybe required and if a Connection can be formed
    /// TODO calculate the real from type based on the subroute of the output used by
    /// the connection from_route
    fn two_compatible_types(from: &DataType, to: &DataType, _from_route: &Route) -> bool {
        if from == to {
            return true;
        }

        if to.is_generic() {
            return true;
        }

        if to.array_of(from) {
            return true;
        }

        if to.array_of(&DataType::from(OBJECT_TYPE)) {
            return true;
        }

        if from.array_of(to) {
            return true;
        }

        // Faith for now!
        if from.is_generic() && !to.array_of(&DataType::from(OBJECT_TYPE)) {
            return true;
        }

        // Faith for now!
        if from.array_of(&DataType::from(OBJECT_TYPE)) && !to.is_array() {
            return true;
        }

        // Faith that OBJECT_TYPE elements can be converted to whatever the destination array is
        if from.array_of(&DataType::from(OBJECT_TYPE)) && to.is_array() {
            return true;
        }

        false
    }
}

#[cfg(test)]
mod test {
    use url::Url;

    use crate::deserializers::deserializer::get_deserializer;
    use crate::errors::*;
    use crate::model::validation::Validate;

    use super::Connection;

    fn toml_from_str(content: &str) -> Result<Connection> {
        let url = Url::parse("file:///fake.toml").expect("Could not parse URL");
        let deserializer =
            get_deserializer::<Connection>(&url).expect("Could not get deserializer");
        deserializer.deserialize(content, Some(&url))
    }

    #[test]
    fn single_destination_deserialization() {
        let input_str = "
        from = 'source'
        to = 'destination'
        ";

        let connection: Result<Connection> = toml_from_str(input_str);
        assert!(connection.is_ok(), "Could not deserialize Connection");
    }

    #[test]
    fn multiple_destination_deserialization() {
        let input_str = "
        from = 'source'
        to = ['destination', 'destination2']
        ";

        let connection: Result<Connection> = toml_from_str(input_str);
        assert!(connection.is_ok(), "Could not deserialize Connection");
    }

    #[test]
    fn display_connection() {
        let connection1 = Connection::new("input/number", "process_1");
        println!("Connection: {}", connection1);
    }

    #[test]
    fn validate_connection() {
        let connection1 = Connection::new("input/number", "process_1");
        assert!(connection1.validate().is_ok());
    }

    #[test]
    fn deserialize_extra_field_fails() {
        let input_str = "
        name = 'input'
        foo = 'extra token'
        type = 'object'
        ";

        let connection: Result<Connection> = toml_from_str(input_str);
        assert!(
            connection.is_err(),
            "Deserialized invalid connection TOML without error, but should not have."
        );
    }

    mod type_conversion {
        use crate::model::datatype::{ARRAY_TYPE, DataType, NUMBER_TYPE, OBJECT_TYPE, STRING_TYPE};
        use crate::model::io::IO;
        use crate::model::route::Route;

        use super::super::Connection;

        /// # Array serialization and object to array wrapping
        ///
        /// ## object being sent
        ///   Value Type   Input Type
        /// * object   --> object (is_array = false) - input and sent to the function as an array
        /// * object   --> array (is_array = true)   - object will be converted to a one element array
        ///
        /// ## array of object being sent
        ///   Value Type        Input Type
        /// * array/object  --> array (is_array = true)
        /// * array/object  --> object (is_array = false) - values in Array will be serialized
        ///                     and sent to input one by one)

        #[test]
        fn type_conversions() {
            let valid_type_conversions: Vec<(String, String)> = vec![
                (NUMBER_TYPE.into(), OBJECT_TYPE.into()),
                (OBJECT_TYPE.into(), OBJECT_TYPE.into()),
                (NUMBER_TYPE.into(), NUMBER_TYPE.into()),
                (NUMBER_TYPE.into(), OBJECT_TYPE.into()),
                (OBJECT_TYPE.into(), NUMBER_TYPE.into()), // Trust me!
                (NUMBER_TYPE.into(), format!("{}/{}", ARRAY_TYPE, OBJECT_TYPE)),
                (NUMBER_TYPE.into(), format!("{}/{}", ARRAY_TYPE, NUMBER_TYPE)),
                (format!("{}/{}", ARRAY_TYPE, OBJECT_TYPE), OBJECT_TYPE.into()),
                (format!("{}/{}", ARRAY_TYPE, NUMBER_TYPE), OBJECT_TYPE.into()),
                (format!("{}/{}", ARRAY_TYPE, NUMBER_TYPE), NUMBER_TYPE.into()),
                (format!("{}/{}", ARRAY_TYPE, NUMBER_TYPE), format!("{}/{}", ARRAY_TYPE, NUMBER_TYPE)),
                (format!("{}/{}", ARRAY_TYPE, OBJECT_TYPE), format!("{}/{}", ARRAY_TYPE, NUMBER_TYPE)),
                (format!("{}/{}", ARRAY_TYPE, OBJECT_TYPE), format!("{}/{}/{}", ARRAY_TYPE, ARRAY_TYPE, NUMBER_TYPE)),
                (format!("{}/{}/{}", ARRAY_TYPE, ARRAY_TYPE, NUMBER_TYPE), OBJECT_TYPE.into()),
                (format!("{}/{}/{}", ARRAY_TYPE, ARRAY_TYPE, NUMBER_TYPE), format!("{}/{}", ARRAY_TYPE, NUMBER_TYPE)),
            ];

            for test in valid_type_conversions.iter() {
                assert!(Connection::compatible_types(
                    &[DataType::from(&test.0 as &str)],
                    &[DataType::from(&test.1 as &str)],
                    &Route::default()
                ));
            }
        }
        #[test]
        fn simple_to_simple() {
            let from_io = IO::new(vec!(STRING_TYPE.into()), "/p1/output");
            let to_io = IO::new(vec!(STRING_TYPE.into()), "/p2");
            assert!(Connection::compatible_types(
                from_io.datatypes(),
                to_io.datatypes(),
                &Route::default()
            ));
        }

        #[test]
        fn simple_indexed_to_simple() {
            let from_io = IO::new(vec!(STRING_TYPE.into()), "/p1/output/0");
            let to_io = IO::new(vec!(STRING_TYPE.into()), "/p2");
            assert!(Connection::compatible_types(
                from_io.datatypes(),
                to_io.datatypes(),
                &Route::default()
            ));
        }

        #[test]
        fn simple_to_simple_mismatch() {
            let from_io = IO::new(vec!(STRING_TYPE.into()), "/p1/output");
            let to_io = IO::new(vec!(NUMBER_TYPE.into()), "/p2");
            assert!(!Connection::compatible_types(
                from_io.datatypes(),
                to_io.datatypes(),
                &Route::default()
            ));
        }

        #[test]
        fn simple_indexed_to_array() {
            let from_io = IO::new(vec!(STRING_TYPE.into()), "/p1/output/0");
            let to_io = IO::new(vec!("array/string".into()), "/p2");
            assert!(Connection::compatible_types(
                from_io.datatypes(),
                to_io.datatypes(),
                &Route::default()
            ));
        }

        #[test]
        fn simple_to_array() {
            let from_io = IO::new(vec!(STRING_TYPE.into()), "/p1/output");
            let to_io = IO::new(vec!("array/string".into()), "/p2");
            assert!(Connection::compatible_types(
                from_io.datatypes(),
                to_io.datatypes(),
                &Route::default()
            ));
        }

        #[test]
        fn simple_to_array_mismatch() {
            let from_io = IO::new(vec!(STRING_TYPE.into()), "/p1/output");
            let to_io = IO::new(vec!("array/number".into()), "/p2");
            assert!(!Connection::compatible_types(
                from_io.datatypes(),
                to_io.datatypes(),
                &Route::default()
            ));
        }

        #[test]
        fn array_to_array() {
            let from_io = IO::new(vec!(ARRAY_TYPE.into()), "/p1/output");
            let to_io = IO::new(vec!(ARRAY_TYPE.into()), "/p2");
            assert!(Connection::compatible_types(
                from_io.datatypes(),
                to_io.datatypes(),
                &Route::default()
            ));
        }

        #[test]
        fn array_to_simple() {
            let from_io = IO::new(vec!("array/string".into()), "/p1/output");
            let to_io = IO::new(vec!(STRING_TYPE.into()), "/p2");
            assert!(Connection::compatible_types(
                from_io.datatypes(),
                to_io.datatypes(),
                &Route::default()
            ));
        }

        #[test]
        fn multiple_output_type_to_single_input_type() {
            let from_io = IO::new(vec!(STRING_TYPE.into(), NUMBER_TYPE.into()), "/p1/output");
            let to_io = IO::new(vec!(STRING_TYPE.into()), "/p2");
            assert!(!Connection::compatible_types(
                from_io.datatypes(),
                to_io.datatypes(),
                &Route::default()
            ));
        }

        #[test]
        fn multiple_output_type_to_value_input_type() {
            let from_io = IO::new(vec!(STRING_TYPE.into(), NUMBER_TYPE.into()), "/p1/output");
            let to_io = IO::new(vec!(OBJECT_TYPE.into()), "/p2");
            assert!(Connection::compatible_types(
                from_io.datatypes(),
                to_io.datatypes(),
                &Route::default()
            ));
        }

        #[test]
        fn multiple_output_type_to_matching_input_types() {
            let from_io = IO::new(vec!(STRING_TYPE.into(), NUMBER_TYPE.into()), "/p1/output");
            let to_io = IO::new(vec!(STRING_TYPE.into(), NUMBER_TYPE.into()), "/p2");
            assert!(Connection::compatible_types(
                from_io.datatypes(),
                to_io.datatypes(),
                &Route::default()
            ));
        }

        #[test]
        fn single_output_type_to_superset_input_types() {
            let from_io = IO::new(vec!(STRING_TYPE.into()), "/p1/output");
            let to_io = IO::new(vec!(STRING_TYPE.into(), NUMBER_TYPE.into()), "/p2");
            assert!(Connection::compatible_types(
                from_io.datatypes(),
                to_io.datatypes(),
                &Route::default()
            ));
        }

        #[test]
        fn multiple_output_type_to_superset_input_types() {
            let from_io = IO::new(vec!(STRING_TYPE.into(), NUMBER_TYPE.into()), "/p1/output");
            let to_io = IO::new(vec!(STRING_TYPE.into(), NUMBER_TYPE.into(), ARRAY_TYPE.into()), "/p2");
            assert!(Connection::compatible_types(
                from_io.datatypes(),
                to_io.datatypes(),
                &Route::default()
            ));
        }

        #[test]
        fn multiple_output_type_to_non_matching_input_types() {
            let from_io = IO::new(vec!(STRING_TYPE.into(), NUMBER_TYPE.into()), "/p1/output");
            let to_io = IO::new(vec!(STRING_TYPE.into(), ARRAY_TYPE.into()), "/p2");
            assert!(!Connection::compatible_types(
                from_io.datatypes(),
                to_io.datatypes(),
                &Route::default()
            ));
        }

        #[test]
        fn single_output_type_to_non_matching_input_types() {
            let from_io = IO::new(vec!(STRING_TYPE.into()), "/p1/output");
            let to_io = IO::new(vec!(ARRAY_TYPE.into(), NUMBER_TYPE.into()), "/p2");
            assert!(!Connection::compatible_types(
                from_io.datatypes(),
                to_io.datatypes(),
                &Route::default()
            ));
        }

        #[test]
        fn multiple_output_type_to_value_input_types() {
            let from_io = IO::new(vec!(STRING_TYPE.into(), NUMBER_TYPE.into()), "/p1/output");
            let to_io = IO::new(vec!(ARRAY_TYPE.into(), OBJECT_TYPE.into()), "/p2");
            assert!(Connection::compatible_types(
                from_io.datatypes(),
                to_io.datatypes(),
                &Route::default()
            ));
        }

        #[test]
        fn single_output_type_to_value_input_types() {
            let from_io = IO::new(vec!(STRING_TYPE.into()), "/p1/output");
            let to_io = IO::new(vec!(ARRAY_TYPE.into(), OBJECT_TYPE.into()), "/p2");
            assert!(Connection::compatible_types(
                from_io.datatypes(),
                to_io.datatypes(),
                &Route::default()
            ));
        }

        #[test]
        fn null_output_type_to_valid_input_types() {
            let from_io = IO::new(vec!(), "/p1/output");
            let to_io = IO::new(vec!(OBJECT_TYPE.into()), "/p2");
            assert!(!Connection::compatible_types(
                from_io.datatypes(),
                to_io.datatypes(),
                &Route::default()
            ));
        }

        #[test]
        fn valid_output_type_to_null_input_types() {
            let from_io = IO::new(vec!(OBJECT_TYPE.into()), "/p1/output");
            let to_io = IO::new(vec!(), "/p2");
            assert!(!Connection::compatible_types(
                from_io.datatypes(),
                to_io.datatypes(),
                &Route::default()
            ));
        }

        #[test]
        fn null_output_type_to_null_input_types() {
            let from_io = IO::new(vec!(), "/p1/output");
            let to_io = IO::new(vec!(), "/p2");
            assert!(!Connection::compatible_types(
                from_io.datatypes(),
                to_io.datatypes(),
                &Route::default()
            ));
        }
    }
}

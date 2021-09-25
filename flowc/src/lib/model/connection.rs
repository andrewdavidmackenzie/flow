use std::fmt;

use serde_derive::{Deserialize, Serialize};

use crate::compiler::loader::Validate;
use crate::errors::*;
use crate::model::datatype::DataType;
use crate::model::io::IO;
use crate::model::name::Name;
use crate::model::route::HasRoute;
use crate::model::route::Route;

/// `Connection` defines a connection between the output of one function or flow to the input
/// of another function or flow and maybe optionally named for legibility/debugging.
#[derive(Deserialize, Serialize, Default, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct Connection {
    /// Optional name given to a connection for legibility and debugging
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: Name,
    /// `from` defines the origin of the connection
    pub from: Route,
    /// `to` defines the destination(s) of this connection
    #[serde(deserialize_with = "super::route_array_serde::route_or_route_array")]
    pub to: Vec<Route>,
    /// `from_io` is used during the compilation process and refers to a found output for the connection
    // TODO make these references, not clones
    #[serde(skip)]
    pub from_io: IO,
    /// `to_io` is used during the compilation process and refers to a found input for the connection
    #[serde(skip)]
    pub to_io: IO,
    /// `level` defines at what level in the flow hierarchy of nested flows this connections belongs
    #[serde(skip)]
    pub level: usize,
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
    /// Determine if the type of the source of a connection and the type of the destination are
    /// compatible, what type of conversion maybe required and if a Connection can be formed
    /// TODO calculate the real from type based on the subroute of the output used by
    /// the connection from_route
    pub fn compatible_types(from: &DataType, to: &DataType, _from_route: &Route) -> bool {
        if from == to {
            return true;
        }

        if to.is_generic() {
            return true;
        }

        if to.array_of(from) {
            return true;
        }

        if to.array_of(&DataType::from("Value")) {
            return true;
        }

        if from.array_of(to) {
            return true;
        }

        // Faith for now!
        if from.is_generic() && !to.array_of(&DataType::from("Value")) {
            return true;
        }

        // Faith for now!
        if from.array_of(&DataType::from("Value")) && !to.is_array() {
            return true;
        }

        // Faith that "Value" elements can be converted to whatever the destination array is
        if from.array_of(&DataType::from("Value")) && to.is_array() {
            return true;
        }

        false
    }
}

#[cfg(test)]
mod test {
    use url::Url;

    use flowcore::deserializers::deserializer::get_deserializer;
    use flowcore::errors::*;

    use crate::compiler::loader::Validate;

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
        let connection1 = Connection {
            from: "input/number".into(),  // Number
            to: vec!["process_1".into()], // String
            ..Default::default()
        };

        println!("Connection: {}", connection1);
    }

    #[test]
    fn validate_connection() {
        let connection1 = Connection {
            from: "input/number".into(),  // Number
            to: vec!["process_1".into()], // String
            ..Default::default()
        };

        assert!(connection1.validate().is_ok());
    }

    #[test]
    fn deserialize_extra_field_fails() {
        let input_str = "
        name = 'input'
        foo = 'extra token'
        type = 'Value'
        ";

        let connection: Result<Connection> = toml_from_str(input_str);
        assert!(
            connection.is_err(),
            "Deserialized invalid connection TOML without error, but should not have."
        );
    }

    mod type_conversion {
        use crate::model::connection::Connection;
        use crate::model::datatype::DataType;
        use crate::model::io::IO;
        use crate::model::route::Route;

        /// # Compatible Types in a Connection
        ///
        /// ## Simple Object Value being sent
        ///   Value Type        Input Type
        /// * Simple Object --> Simple Object (is_array = false)
        /// input and sent to the function as an array
        /// * Simple Object --> Array (is_array = true)
        ///
        /// ## Array Object being sent
        ///   Value Type        Input Type
        /// * Array Object  --> Array (is_array = true)
        /// * Array Object  --> Simple Object (is_array = false) (values in Array will be
        /// serialized and sent to input one by one)

        #[test]
        fn type_conversions() {
            let valid_types: Vec<(&str, &str)> = vec![
                ("Number", "Value"),
                ("Value", "Value"),
                ("Array/Value", "Value"),
                ("Number", "Number"),
                ("Number", "Value"),
                ("Array/Number", "Value"),
                ("Number", "Array/Number"),
                ("Array/Number", "Number"),
                ("Number", "Array/Value"),
                ("Array/Number", "Array/Number"),
                ("Array/Value", "Array/Array/Number"),
                ("Array/Array/Number", "Array/Number"),
                ("Array/Array/Number", "Value"),
                ("Value", "Number"), // Trust me!
                ("Array/Value", "Array/Number"),
            ];

            for test in valid_types.iter() {
                assert!(Connection::compatible_types(
                    &DataType::from(test.0),
                    &DataType::from(test.1),
                    &Route::default()
                ));
            }
        }
        #[test]
        fn simple_to_simple() {
            let from_io = IO::new("String", "/p1/output");
            let to_io = IO::new("String", "/p2");
            assert!(Connection::compatible_types(
                from_io.datatype(),
                to_io.datatype(),
                &Route::default()
            ));
        }

        #[test]
        fn simple_indexed_to_simple() {
            let from_io = IO::new("String", "/p1/output/0");
            let to_io = IO::new("String", "/p2");
            assert!(Connection::compatible_types(
                from_io.datatype(),
                to_io.datatype(),
                &Route::default()
            ));
        }

        #[test]
        fn simple_to_simple_mismatch() {
            let from_io = IO::new("String", "/p1/output");
            let to_io = IO::new("Number", "/p2");
            assert!(!Connection::compatible_types(
                from_io.datatype(),
                to_io.datatype(),
                &Route::default()
            ));
        }

        #[test]
        fn simple_indexed_to_array() {
            let from_io = IO::new("String", "/p1/output/0");
            let to_io = IO::new("Array/String", "/p2");
            assert!(Connection::compatible_types(
                from_io.datatype(),
                to_io.datatype(),
                &Route::default()
            ));
        }

        #[test]
        fn simple_to_array() {
            let from_io = IO::new("String", "/p1/output");
            let to_io = IO::new("Array/String", "/p2");
            assert!(Connection::compatible_types(
                from_io.datatype(),
                to_io.datatype(),
                &Route::default()
            ));
        }

        #[test]
        fn simple_to_array_mismatch() {
            let from_io = IO::new("String", "/p1/output");
            let to_io = IO::new("Array/Number", "/p2");
            assert!(!Connection::compatible_types(
                from_io.datatype(),
                to_io.datatype(),
                &Route::default()
            ));
        }

        #[test]
        fn array_to_array() {
            let from_io = IO::new("Array", "/p1/output");
            let to_io = IO::new("Array", "/p2");
            assert!(Connection::compatible_types(
                from_io.datatype(),
                to_io.datatype(),
                &Route::default()
            ));
        }

        #[test]
        fn array_to_simple() {
            let from_io = IO::new("Array/String", "/p1/output");
            let to_io = IO::new("String", "/p2");
            assert!(Connection::compatible_types(
                from_io.datatype(),
                to_io.datatype(),
                &Route::default()
            ));
        }
    }
}

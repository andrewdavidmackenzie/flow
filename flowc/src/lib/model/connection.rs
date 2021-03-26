use std::fmt;

use serde_derive::{Deserialize, Serialize};

use crate::compiler::loader::Validate;
use crate::errors::*;
use crate::model::datatype::DataType;
use crate::model::io::IO;
use crate::model::name::Name;
use crate::model::route::HasRoute;
use crate::model::route::Route;

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct Connection {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<Name>,
    pub from: Route,
    pub to: Route,

    // TODO make these references, not clones
    #[serde(skip)]
    pub from_io: IO,
    #[serde(skip)]
    pub to_io: IO,
    #[serde(skip)]
    pub level: usize,
}

#[derive(Debug)]
#[allow(clippy::upper_case_acronyms)]
pub enum Direction {
    FROM,
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
    // Called before everything is loaded and connected up to check all looks good
    fn validate(&self) -> Result<()> {
        if let Some(ref name) = self.name {
            name.validate()?;
        }
        self.from.validate()?;
        self.to.validate()
    }
}

impl Connection {
    /*
        Determine if the type of the source of a connection and the type of the destination are
        compatible, what type of conversion maybe required and if a Connection can be formed
    */
    pub fn compatible_types(from: &DataType, to: &DataType) -> bool {
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
    use crate::model::datatype::DataType;
    use crate::model::io::IO;

    use super::Connection;

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
                &DataType::from(test.1)
            ));
        }
    }

    #[test]
    fn deserialize_simple() {
        let input_str = "
        from = 'source'
        to = 'dest'
        ";

        let _connection: Connection = toml::from_str(input_str).unwrap();
    }

    #[test]
    fn deserialize_extra_field_fails() {
        let input_str = "
        name = 'input'
        foo = 'extra token'
        type = 'Value'
        ";

        let connection: Result<Connection, _> = toml::from_str(input_str);
        assert!(connection.is_err());
    }

    /******************** Tests for compatible_types ********************/

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
    fn simple_to_simple() {
        let from_io = IO::new("String", "/p1/output");
        let to_io = IO::new("String", "/p2");
        assert!(Connection::compatible_types(
            &from_io.datatype(),
            &to_io.datatype()
        ));
    }

    #[test]
    fn simple_indexed_to_simple() {
        let from_io = IO::new("String", "/p1/output/0");
        let to_io = IO::new("String", "/p2");
        assert!(Connection::compatible_types(
            &from_io.datatype(),
            &to_io.datatype()
        ));
    }

    #[test]
    fn simple_to_simple_mismatch() {
        let from_io = IO::new("String", "/p1/output");
        let to_io = IO::new("Number", "/p2");
        assert_eq!(
            Connection::compatible_types(&from_io.datatype(), &to_io.datatype()),
            false
        );
    }

    #[test]
    fn simple_indexed_to_array() {
        let from_io = IO::new("String", "/p1/output/0");
        let to_io = IO::new("Array/String", "/p2");
        assert!(Connection::compatible_types(
            &from_io.datatype(),
            &to_io.datatype()
        ));
    }

    #[test]
    fn simple_to_array() {
        let from_io = IO::new("String", "/p1/output");
        let to_io = IO::new("Array/String", "/p2");
        assert!(Connection::compatible_types(
            &from_io.datatype(),
            &to_io.datatype()
        ));
    }

    #[test]
    fn simple_to_array_mismatch() {
        let from_io = IO::new("String", "/p1/output");
        let to_io = IO::new("Array/Number", "/p2");
        assert_eq!(
            Connection::compatible_types(&from_io.datatype(), &to_io.datatype()),
            false
        );
    }

    #[test]
    fn array_to_array() {
        let from_io = IO::new("Array", "/p1/output");
        let to_io = IO::new("Array", "/p2");
        assert!(Connection::compatible_types(
            &from_io.datatype(),
            &to_io.datatype()
        ));
    }

    #[test]
    fn array_to_simple() {
        let from_io = IO::new("Array/String", "/p1/output");
        let to_io = IO::new("String", "/p2");
        assert!(Connection::compatible_types(
            &from_io.datatype(),
            &to_io.datatype()
        ));
    }
}

use std::fmt;

use serde_derive::{Deserialize, Serialize};

use flowrlib::output_connection::Conversion;
use flowrlib::output_connection::Conversion::{ArraySerialize, WrapAsArray};

use crate::compiler::loader::Validate;
use crate::errors::*;
use crate::model::datatype::{DataType, TypeCheck};
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
    #[serde(skip)]
    pub conversion: Option<Conversion>
}

#[derive(Debug)]
pub enum Direction {
    FROM,
    TO,
}

impl fmt::Display for Connection {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match (self.from_io.flow_io(), self.to_io.flow_io()) {
            (true, true) => write!(f, "(f){} --> {}(f)", self.from_io.route(), self.to_io.route()),
            (true, false) => write!(f, "(f){} --> {}", self.from_io.route(), self.to_io.route()),
            (false, true) => write!(f, "   {} --> {}(f)", self.from_io.route(), self.to_io.route()),
            (false, false) => write!(f, "   {} --> {}", self.from_io.route(), self.to_io.route())
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
    pub fn compatible_types(from: &IO, to: &IO) -> Result<Option<Conversion>> {
        if from.datatype() == to.datatype() {
            return Ok(None);
        }

        if !from.datatype().is_array() && to.datatype().is_generic() {
            return Ok(None);
        }

        if to.datatype().array_of(from.datatype()) {
            return Ok(Some(WrapAsArray));
        }

        if to.datatype().array_of(&DataType::from("Value")) {
            return Ok(Some(WrapAsArray));
        }

        if from.datatype().is_array() && to.datatype().is_generic() {
            return Ok(Some(ArraySerialize));
        }

        if from.datatype().array_of(to.datatype()) {
            return Ok(Some(ArraySerialize));
        }

        // Faith for now!
        if from.datatype().is_generic() && !to.datatype().array_of(&DataType::from("Value")) {
            return Ok(None)
        }

        // Faith for now!
        if from.datatype().array_of(&DataType::from("Value")) && !to.datatype().is_array() {
            return Ok(Some(ArraySerialize));
        }

        bail!("Types cannot be connected: '{}' --> '{}'", from.datatype(), to.datatype())
    }
}

#[cfg(test)]
mod test {
    use crate::model::io::IO;
    use crate::model::route::Route;

    use super::Connection;

    #[test]
    fn no_path_no_change() {
        let route = Route::from("");
        let (new_route, _num, trailing_number) = route.without_trailing_array_index();
        assert_eq!(new_route.into_owned(), Route::default());
        assert_eq!(trailing_number, false);
    }

    #[test]
    fn just_slash_no_change() {
        let route = Route::from("/");
        let (new_route, _num, trailing_number) = route.without_trailing_array_index();
        assert_eq!(new_route.into_owned(), Route::from("/"));
        assert_eq!(trailing_number, false);
    }

    #[test]
    fn no_trailing_number_no_change() {
        let route = Route::from("/output1");
        let (new_route, _num, trailing_number) = route.without_trailing_array_index();
        assert_eq!(new_route.into_owned(), Route::from("/output1"));
        assert_eq!(trailing_number, false);
    }

    #[test]
    fn detect_array_at_output_root() {
        let route = Route::from("/0");
        let (new_route, num, trailing_number) = route.without_trailing_array_index();
        assert_eq!(new_route.into_owned(), Route::from(""));
        assert_eq!(num, 0);
        assert_eq!(trailing_number, true);
    }

    #[test]
    fn detect_array_at_output_subpath() {
        let route = Route::from("/array_output/0");
        let (new_route, num, trailing_number) = route.without_trailing_array_index();
        assert_eq!(new_route.into_owned(), Route::from("/array_output"));
        assert_eq!(num, 0);
        assert_eq!(trailing_number, true);
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
    #[should_panic]
    fn deserialize_extra_field_fails() {
        let input_str = "
        name = 'input'
        foo = 'extra token'
        type = 'Value'
        ";

        let _connection: Connection = toml::from_str(input_str).unwrap();
    }

    /******************** Tests for compatible_types ********************/

    /// # Compatible Types in a Connection
    ///
    /// ## Simple Object Value being sent
    ///   Value Type        Input Type
    /// * Simple Object --> Simple Object (is_array = false, depth = 1)
    /// * Simple Object --> Simple Object (is_array = false, depth > 1) (will be accumulated at the
    /// input and sent to the function as an array of size 'depth'
    /// * Simple Object --> Array (is_array = true, depth = 1)
    ///
    /// ## Array Object being sent
    ///   Value Type        Input Type
    /// * Array Object  --> Array (is_array = true, depth = 1)
    /// * Array Object  --> Array (is_array = true, depth > 1)
    /// * Array Object  --> Simple Object (is_array = false, depth = 1) (values in Array will be
    /// serialized and sent to input one by one, will be extracted one-by-one as per depth)
    /// * Array Object  --> Simple Object (is_array = false, depth > 1) (values in Array will be
    /// serialized and sent to input one by one, will be extracted in sets of size 'depth')
    #[test]
    fn simple_to_simple_depth_1() {
        let from_io = IO::new("String", &Route::from("/p1/output"));
        let to_io = IO::new("String", &Route::from("/p2"));
        assert!(Connection::compatible_types(&from_io, &to_io).is_ok());
    }

    #[test]
    fn simple_indexed_to_simple_depth_1() {
        let from_io = IO::new("String", &Route::from("/p1/output/0"));
        let to_io = IO::new("String", &Route::from("/p2"));
        assert!(Connection::compatible_types(&from_io, &to_io).is_ok());
    }

    #[test]
    fn simple_to_simple_depth_1_mismatch() {
        let from_io = IO::new("String", &Route::from("/p1/output"));
        let to_io = IO::new("Number", &Route::from("/p2"));
        assert!(Connection::compatible_types(&from_io, &to_io).is_err());
    }

    #[test]
    fn simple_indexed_to_array() {
        let from_io = IO::new("String", &Route::from("/p1/output/0"));
        let to_io = IO::new("Array/String", &Route::from("/p2"));
        assert!(Connection::compatible_types(&from_io, &to_io).is_ok());
    }

    #[test]
    fn simple_to_simple_depth_greater_than_1() {
        let from_io = IO::new("String", &Route::from("/p1/output"));
        let mut to_io = IO::new("String", &Route::from("/p2"));
        to_io.set_depth(2);
        assert!(Connection::compatible_types(&from_io, &to_io).is_ok());
    }

    #[test]
    fn simple_to_array() {
        let from_io = IO::new("String", &Route::from("/p1/output"));
        let to_io = IO::new("Array/String", &Route::from("/p2"));
        assert!(Connection::compatible_types(&from_io, &to_io).is_ok());
    }

    #[test]
    fn simple_to_array_mismatch() {
        let from_io = IO::new("String", &Route::from("/p1/output"));
        let to_io = IO::new("Array/Number", &Route::from("/p2"));
        assert!(Connection::compatible_types(&from_io, &to_io).is_err());
    }

    #[test]
    fn array_to_array_depth_1() {
        let from_io = IO::new("Array", &Route::from("/p1/output"));
        let to_io = IO::new("Array", &Route::from("/p2"));
        assert!(Connection::compatible_types(&from_io, &to_io).is_ok());
    }

    #[test]
    fn array_to_array_depth_more_than_1() {
        let from_io = IO::new("Array", &Route::from("/p1/output"));
        let mut to_io = IO::new("Array", &Route::from("/p2"));
        to_io.set_depth(2);
        assert!(Connection::compatible_types(&from_io, &to_io).is_ok());
    }

    #[test]
    fn array_to_simple_depth_1() {
        let from_io = IO::new("Array/String", &Route::from("/p1/output"));
        let to_io = IO::new("String", &Route::from("/p2"));
        assert!(Connection::compatible_types(&from_io, &to_io).is_ok());
    }

    #[test]
    fn array_to_simple_depth_1_mismatch() {
        let from_io = IO::new("Array/Number", &Route::from("/p1/output"));
        let to_io = IO::new("String", &Route::from("/p2"));
        assert!(Connection::compatible_types(&from_io, &to_io).is_err());
    }

    #[test]
    fn array_to_simple_depth_more_than_1() {
        let from_io = IO::new("Array/String", &Route::from("/p1/output"));
        let mut to_io = IO::new("String", &Route::from("/p2"));
        to_io.set_depth(2);
        assert!(Connection::compatible_types(&from_io, &to_io).is_ok());
    }
}
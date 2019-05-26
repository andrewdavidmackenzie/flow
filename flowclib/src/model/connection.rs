use model::name::Name;
use compiler::loader::Validate;
use model::route::Route;
use model::route::HasRoute;
use model::io::IO;
use std::fmt;
use model::datatype::TypeCheck;

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct Connection {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<Name>,
    pub from: Route,
    pub to: Route,

    // TODO make these references, not clones
    #[serde(skip_deserializing)]
    pub from_io: IO,
    #[serde(skip_deserializing)]
    pub to_io: IO,
    #[serde(skip_deserializing)]
    pub level: usize,
}

#[derive(Debug)]
pub enum Direction {
    FROM,
    TO,
}

impl fmt::Display for Connection {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match (self.from_io.flow_io(), self.to_io.flow_io()) {
            (true, true) => write!(f,   "(f){} --> {}(f)", self.from_io.route(), self.to_io.route()),
            (true, false) => write!(f,  "(f){} --> {}", self.from_io.route(), self.to_io.route()),
            (false, true) => write!(f,  "   {} --> {}(f)", self.from_io.route(), self.to_io.route()),
            (false, false) => write!(f, "   {} --> {}", self.from_io.route(), self.to_io.route())
        }
    }
}

impl Validate for Connection {
    // Called before everything is loaded and connected up to check all looks good
    fn validate(&self) -> Result<(), String> {
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
        compatible, and a Connection can be formed that can be implemented by the run-time
    */
    pub fn compatible_types(from: &IO, to: &IO) -> bool {
        from.datatype(0) == to.datatype(0) ||
            from.datatype(0).is_generic() ||
            to.datatype(0).is_generic() ||
            (from.datatype(0).is_array() && from.datatype(1) == to.datatype(0) ||
                (to.datatype(0).is_array() && to.datatype(1) == from.datatype(0)))
    }
}

#[cfg(test)]
mod test {
    use model::route::Route;
    use model::io::IO;
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
        type = 'Json'
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
        let from_io = IO::new("String", &Route::from("/output"));
    }

    #[test]
    fn simple_to_simple_depth_greater_than_1() {

    }

    #[test]
    fn simple_to_array() {

    }

    #[test]
    fn array_to_array_depth_1() {

    }

    #[test]
    fn array_to_array_depth_more_than_1() {

    }

    #[test]
    fn array_to_simple_depth_1() {

    }

    #[test]
    fn array_to_simple_depth_more_than_1() {

    }
}
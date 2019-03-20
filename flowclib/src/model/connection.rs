use model::name::Name;
use compiler::loader::Validate;
use model::route::Route;
use model::route::HasRoute;
use model::io::IO;
use std::fmt;

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
}

#[derive(Debug)]
pub enum Direction {
    FROM,
    TO,
}

impl Connection {
    pub fn check_for_loops(&self, source: &str) -> Result<(), String> {
        if self.from == self.to {
            return Err(format!("Connection loop detected in flow '{}' from '{}' to '{}'",
                               source, self.from, self.to));
        }

        Ok(())
    }
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

#[cfg(test)]
mod test {
    use model::route::Route;
    use model::route::Router;
    use super::Connection;

    #[test]
    fn no_path_no_change() {
        let route = Route::from("");
        let (new_route, _num, trailing_number) = Router::without_trailing_array_index(&route);
        assert_eq!(new_route.as_ref(), "");
        assert_eq!(trailing_number, false);
    }

    #[test]
    fn just_slash_no_change() {
        let route = Route::from("/");
        let (new_route, _num, trailing_number) = Router::without_trailing_array_index(&route);
        assert_eq!(new_route.as_ref(), "/");
        assert_eq!(trailing_number, false);
    }

    #[test]
    fn no_trailing_number_no_change() {
        let route = Route::from("/output1");
        let (new_route, _num, trailing_number) = Router::without_trailing_array_index(&route);
        assert_eq!(new_route.as_ref(), "/output1");
        assert_eq!(trailing_number, false);
    }

    #[test]
    fn detect_array_at_output_root() {
        let route = Route::from("/0");
        let (new_route, num, trailing_number) = Router::without_trailing_array_index(&route);
        assert_eq!(new_route.as_ref(), "");
        assert_eq!(num, 0);
        assert_eq!(trailing_number, true);
    }

    #[test]
    fn detect_array_at_output_subpath() {
        let route = Route::from("/array_output/0");
        let (new_route, num, trailing_number) = Router::without_trailing_array_index(&route);
        assert_eq!(new_route.as_ref(), "/array_output");
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
}
use model::name::Name;
use loader::loader::Validate;
use model::route::Route;
use model::route::HasRoute;
use model::io::IO;
use std::borrow::Cow;
use std::fmt;

#[derive(Deserialize, Debug, Clone)]
pub struct Connection {
    pub name: Option<Name>,
    pub from: Route,
    pub to: Route,

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

/*
    return the io route without a trailing number (array index) and if it has one or not

    If the trailing number was present then return the route with a trailing '/'
*/
pub fn name_without_trailing_number<'a>(route: &'a str) -> (Cow<'a, str>, usize, bool) {
    let mut parts: Vec<&str> = route.split('/').collect();
    if let Some(last_part) = parts.pop() {
        if let Ok(number) = last_part.parse::<usize>() {
            let route_without_number = parts.join("/");
            return (Cow::Owned(route_without_number), number, true);
        }
    }

    (Cow::Borrowed(route), 0, false)
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
        match (self.from_io.flow_io, self.to_io.flow_io) {
            (true, true) => write!(f, "(f){} --> (f){}", self.from_io.route(), self.to_io.route()),
            (true, false) => write!(f, "(f){} --> {}", self.from_io.route(), self.to_io.route()),
            (false, true) => write!(f, "{} --> (f){}", self.from_io.route(), self.to_io.route()),
            (false, false) => write!(f, "{} --> {}", self.from_io.route(), self.to_io.route())
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
    use super::name_without_trailing_number;

    #[test]
    fn no_path_no_change() {
        let route = "";
        let (new_route, _num, trailing_number) = name_without_trailing_number(route);
        assert_eq!(new_route, "");
        assert_eq!(trailing_number, false);
    }

    #[test]
    fn just_slash_no_change() {
        let route = "/";
        let (new_route, _num, trailing_number) = name_without_trailing_number(route);
        assert_eq!(new_route, "/");
        assert_eq!(trailing_number, false);
    }

    #[test]
    fn no_trailing_number_no_change() {
        let route = "/output1";
        let (new_route, _num, trailing_number) = name_without_trailing_number(route);
        assert_eq!(new_route, "/output1");
        assert_eq!(trailing_number, false);
    }

    #[test]
    fn detect_array_at_output_root() {
        let route = "/0";
        let (new_route, num, trailing_number) = name_without_trailing_number(route);
        assert_eq!(new_route, "");
        assert_eq!(num, 0);
        assert_eq!(trailing_number, true);
    }

    #[test]
    fn detect_array_at_output_subpath() {
        let route = "/array_output/0";
        let (new_route, num, trailing_number) = name_without_trailing_number(route);
        assert_eq!(new_route, "/array_output");
        assert_eq!(num, 0);
        assert_eq!(trailing_number, true);
    }
}
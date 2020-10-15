use std::borrow::Cow;
use std::fmt;

use serde_derive::{Deserialize, Serialize};
use shrinkwraprs::Shrinkwrap;

use crate::compiler::loader::Validate;
use crate::errors::*;
use crate::model::io::IOType;
use crate::model::name::Name;

#[derive(Shrinkwrap, Hash, Debug, PartialEq, Clone, Default, Serialize, Deserialize, Eq)]
#[shrinkwrap(mutable)]
pub struct Route(pub String);

impl Route {
    pub fn sub_route_of(&self, other_route: &Route) -> Option<Route> {
        if self == other_route {
            Some(Route::from(""))
        } else if self.as_str().starts_with(&format!("{}/", other_route.as_str())) {
            Some(Route::from(&self.as_str()[other_route.len()..]))
        } else {
            None
        }
    }

    pub fn insert(&mut self, sub_route: &Route) -> &Self {
        self.insert_str(0, sub_route.as_str());
        self
    }

    pub fn push(&mut self, sub_route: &Route) -> &Self {
        self.push_str(sub_route.as_str());
        self
    }

    /*
        Return a route that is one level up, such that
            /context/function/output/subroute -> /context/function/output
     */
    pub fn pop(&self) -> (Cow<Route>, Option<Route>) {
        let mut parts: Vec<&str> = self.split('/').collect();
        let sub_route = parts.pop();
        match sub_route {
            None => (Cow::Borrowed(self), None),
            Some("") => (Cow::Borrowed(self), None),
            Some(sr) => (Cow::Owned(Route::from(parts.join("/"))), Some(Route::from(sr)))
        }
    }

    /*
        Return the io route without a trailing number (array index) and if it has one or not
        If the trailing number was present then return the route with a trailing '/'
    */
    pub fn without_trailing_array_index(&self) -> (Cow<Route>, usize, bool) {
        let mut parts: Vec<&str> = self.split('/').collect();
        if let Some(last_part) = parts.pop() {
            if let Ok(index) = last_part.parse::<usize>() {
                let route_without_number = parts.join("/");
                return (Cow::Owned(Route::from(route_without_number)), index, true);
            }
        }

        (Cow::Borrowed(self), 0, false)
    }
}

impl Validate for Route {
    fn validate(&self) -> Result<()> {
        if self.is_empty() {
            return Ok(());
        }

        if self.parse::<usize>().is_ok() {
            bail!("Route '{}' is invalid - cannot be an integer", self);
        }

        Ok(())
    }
}

pub trait HasRoute {
    fn route(&self) -> &Route;
}

pub trait SetRoute {
    fn set_routes_from_parent(&mut self, parent: &Route);
}

pub trait SetIORoutes {
    fn set_io_routes_from_parent(&mut self, parent: &Route, io_type: IOType);
}

impl fmt::Display for Route {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for Route {
    fn from(string: &str) -> Self {
        Route(string.to_string())
    }
}

impl From<&String> for Route {
    fn from(string: &String) -> Self {
        Route(string.to_string())
    }
}

impl From<String> for Route {
    fn from(string: String) -> Self {
        Route(string)
    }
}

impl From<&Name> for Route {
    fn from(name: &Name) -> Self {
        Route(name.to_string())
    }
}

#[cfg(test)]
mod test {
    use crate::compiler::loader::Validate;

    use super::Route;

    #[test]
    fn test_route_pop() {
        let original = Route::from("/context/function/output/subroute");
        let (level_up, sub) = original.pop();
        assert_eq!(level_up.into_owned(), Route::from("/context/function/output"));
        assert_eq!(sub, Some(Route::from("subroute")));
    }

    #[test]
    fn test_root_route_pop() {
        let original = Route::from("/");
        let (level_up, sub) = original.pop();
        assert_eq!(level_up.into_owned(), Route::from("/"));
        assert_eq!(sub, None);
    }

    #[test]
    fn test_empty_route_pop() {
        let original = Route::from("");
        let (level_up, sub) = original.pop();
        assert_eq!(level_up.into_owned(), Route::from(""));
        assert_eq!(sub, None);
    }

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
    fn detect_array_at_output_subroute() {
        let route = Route::from("/array_output/0");
        let (new_route, num, trailing_number) = route.without_trailing_array_index();
        assert_eq!(new_route.into_owned(), Route::from("/array_output"));
        assert_eq!(num, 0);
        assert_eq!(trailing_number, true);
    }

    #[test]
    fn validate_empty_route() {
        let route = Route::from("");
        assert!(route.validate().is_ok());
    }

    #[test]
    fn validate_root_route() {
        let route = Route::from("/");
        assert!(route.validate().is_ok());
    }

    #[test]
    fn validate_route() {
        let route = Route::from("/context/f1");
        assert!(route.validate().is_ok());
    }

    #[test]
    fn validate_invalid_route() {
        let route = Route::from("123");
        assert!(route.validate().is_err());
    }

    #[test]
    fn subroute_equal_route() {
        let route = Route::from("/context/function");
        assert!(route.sub_route_of(&Route::from("/context/function")).is_some())
    }

    #[test]
    fn subroute_distinct_route() {
        let route = Route::from("/context/function");
        assert!(!route.sub_route_of(&Route::from("/context/foo")).is_some())
    }

    #[test]
    fn subroute_extended_name_route() {
        let route = Route::from("/context/function_foo");
        assert!(!route.sub_route_of(&Route::from("/context/function")).is_some())
    }

    #[test]
    fn is_a_subroute() {
        let route = Route::from("/context/function/input");
        assert!(route.sub_route_of(&Route::from("/context/function")).is_some())
    }

    #[test]
    fn is_a_sub_subroute() {
        let route = Route::from("/context/function/input/element");
        assert!(route.sub_route_of(&Route::from("/context/function")).is_some())
    }

    #[test]
    fn is_array_element_subroute() {
        let route = Route::from("/context/function/1");
        assert!(route.sub_route_of(&Route::from("/context/function")).is_some())
    }

    #[test]
    fn is_array_element_sub_subroute() {
        let route = Route::from("/context/function/input/1");
        assert!(route.sub_route_of(&Route::from("/context/function")).is_some())
    }
}
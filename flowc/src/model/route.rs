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
    pub fn sub_route_of(&self, other_route: &Route) -> bool {
        self.as_str().starts_with(other_route.as_str())
    }

    pub fn push(&mut self, sub_route: &Route) {
        self.push_str(sub_route.as_str());
    }

    /*
        Return a route that is one level up, such that
            /context/function/output/subroute -> /context/function/output
     */
    pub fn pop(&self) -> (Route, Option<Route>) {
        let mut parts: Vec<&str> = self.split('/').collect();
        let sub_route = parts.pop();
        match sub_route {
            None => (self.clone(), None),
            Some("") => (self.clone(), None),
            Some(sr) => (Route::from(parts.join("/")), Some(Route::from(sr)))
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
    use super::Route;

    #[test]
    fn test_route_pop() {
        let original = Route::from("/context/function/output/subroute");
        let (level_up, sub) = original.pop();
        assert_eq!(level_up, Route::from("/context/function/output"));
        assert_eq!(sub, Some(Route::from("subroute")));
    }

    #[test]
    fn test_root_route_pop() {
        let original = Route::from("/");
        let (level_up, sub) = original.pop();
        assert_eq!(level_up, Route::from("/"));
        assert_eq!(sub, None);
    }

    #[test]
    fn test_empty_route_pop() {
        let original = Route::from("");
        let (level_up, sub) = original.pop();
        assert_eq!(level_up, Route::from(""));
        assert_eq!(sub, None);
    }
}
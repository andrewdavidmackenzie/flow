use std::borrow::Cow;
use std::fmt;

use crate::compiler::loader::Validate;
use crate::model::io::IOType;
use crate::model::name::Name;
use crate::errors::*;

#[derive(Shrinkwrap, Hash, Debug, PartialEq, Clone, Default, Serialize, Deserialize, Eq)]
pub struct Route(String);

impl Route {
    pub fn sub_route_of(&self, other_route: &Route) -> bool {
        self.as_str().starts_with(other_route.as_str())
    }

    pub fn push(&mut self, sub_route: &Route) {
        self.to_string().push_str(sub_route.as_str());
    }

    /*
        Return the io route without a trailing number (array index) and if it has one or not
        If the trailing number was present then return the route with a trailing '/'
    */
    pub fn without_trailing_array_index(&self) -> (Cow<Route>, usize, bool) {
        let mut parts: Vec<&str> = self.split('/').collect();
        if let Some(last_part) = parts.pop() {
            if let Ok(number) = last_part.parse::<usize>() {
                let route_without_number = parts.join("/");
                return (Cow::Owned(Route::from(route_without_number)), number, true);
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

        /*
        if !self.starts_with('/') {
            return Err(format!("Non-empty route '{}' must start with '/'", self));
        }
        */

        Ok(())
    }
}

pub trait HasRoute {
    fn route(&self) -> &Route;
}

pub trait FindRoute {
    fn find(&self, route: &Route) -> bool;
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
        Route(string.to_string())
    }
}

impl From<&Name> for Route {
    fn from(name: &Name) -> Self {
        Route(name.to_string())
    }
}
use std::borrow::Cow;
use std::fmt;

use serde_derive::{Deserialize, Serialize};
use shrinkwraprs::Shrinkwrap;

use crate::errors::*;
use crate::model::io::IOType;
use crate::model::name::Name;
use crate::model::validation::Validate;

/// A `Route` is a String that refers to a particular location within the flow hierarchy
/// and can be used to locate a function, flow, input or output uniquely
#[derive(Shrinkwrap, Hash, Debug, PartialEq, Clone, Default, Serialize, Deserialize, Eq)]
#[shrinkwrap(mutable)]
pub struct Route(pub String);

/// A `Route` can be of various Types
pub enum RouteType {
    /// The route refers to an Input of a Flow
    FlowInput(Name, Route),
    /// The Route refers to the Output of a Flow
    FlowOutput(Name),
    /// The route specifies a sub-process of a flow (Input or Output)
    SubProcess(Name, Route)
}

/// `Route` is used to locate Processes (Flows or Functions), their IOs and sub-elements of a
/// data structure within the flow hierarchy
///
/// Examples
/// "/my-flow" -> The flow called "my-flow, anchored at the root of the hierarchy, i.e. the context
/// "/my-flow/sub-flow" -> A flow called "sub-flow" that is within "my-flow"
/// "/my-flow/sub-flow/function" -> A function called "function" within "sub-flow"
/// "/my-flow/sub-flow/function/input_1" -> An IO called "input_1" of "function"
/// "/my-flow/sub-flow/function/input_1/1" -> An array element at index 1 of the Array output from "input_1"
/// "/my-flow/sub-flow/function/input_2/part_a" -> A part of the Json structure output by "input_2" called "part_a"
impl Route {
    /// `sub_route_of` returns an Option<Route> indicating if `self` is a subroute of `other`
    /// (i.e. `self` is a longer route to an element under the `other` route)
    /// Return values
    ///     None                    - `self` is not a sub-route of `other`
    ///     (e.g. ("/my-route1", "/my-route2")
    ///     (e.g. ("/my-route1", "/my-route1/something")
    ///     Some(Route::from(""))   - `self` and `other` are equal
    ///     (e.g. ("/my-route1", "/my-route1")
    ///     Some(Route::from(diff)) - `self` is a sub-route of `other` - with `diff` added
    ///     (e.g. ("/my-route1/something", "/my-route1")
    pub fn sub_route_of(&self, other: &Route) -> Option<Route> {
        if self == other {
            return Some(Route::from(""));
        }

        self.strip_prefix(&format!("{}/", other)).map(Route::from)
    }

    /// Insert another Route at the front of this Route
    pub fn insert<R: AsRef<str>>(&mut self, sub_route: R) -> &Self {
        self.insert_str(0, sub_route.as_ref());
        self
    }

    /// Extend a Route by appending another Route to the end, adding the '/' separator if needed
    pub fn extend(&mut self, sub_route: &Route) -> &Self {
        if !sub_route.is_empty() {
            if !self.to_string().ends_with('/') && !sub_route.starts_with('/') {
                self.push('/');
            }
            self.push_str(sub_route);
        }

        self
    }

    /// Return the type of this Route
    pub fn route_type(&self) -> Result<RouteType> {
        let segments: Vec<&str> = self.split('/').collect();

        match segments[0] {
            "input" => Ok(RouteType::FlowInput(segments[1].into(),
                                               segments[2..].join("/").into())),
            "output" => Ok(RouteType::FlowOutput(segments[1].into())),
            "" => bail!("Invalid route '{}' - 'input' or 'output' or a valid sub-process name \
                must be specified in the route", self),
            process_name => Ok(RouteType::SubProcess(process_name.into(),
                                                     segments[1..].join("/").into())),
        }
    }

    /// Return a route that is one level up, such that
    ///     `/context/function/output/subroute -> /context/function/output`
    pub fn pop(&self) -> (Cow<Route>, Option<Route>) {
        let mut segments: Vec<&str> = self.split('/').collect();
        let sub_route = segments.pop();
        match sub_route {
            None => (Cow::Borrowed(self), None),
            Some("") => (Cow::Borrowed(self), None),
            Some(sr) => (
                Cow::Owned(Route::from(segments.join("/"))),
                Some(Route::from(sr)),
            ),
        }
    }

    /// Return the io route without a trailing number (array index) and if it has one or not
    /// If the trailing number was present then return the route with a trailing '/'
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

    /// Return true if the route selects an element from an array
    pub fn is_array_selector(&self) -> bool {
        if self.is_empty() {
            return false;
        }

        let mut parts: Vec<&str> = self.split('/').collect();
        if let Some(last_part) = parts.pop() {
            return last_part.parse::<usize>().is_ok();
        }

        false
    }
}

impl AsRef<str> for Route {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Validate for Route {
    fn validate(&self) -> Result<()> {
        self.route_type()?;

        if self.parse::<usize>().is_ok() {
            bail!("Route '{}' is invalid - cannot be an integer", self);
        }

        Ok(())
    }
}

/// A trait implemented by objects that have Routes
pub trait HasRoute {
    /// Return a reference to the Route of the struct that implements this trait
    fn route(&self) -> &Route;
    /// Return a mutable reference to the Route of the struct that implements this trait
    fn route_mut(&mut self) -> &mut Route;
}

/// Some structs with Routes will be able to have their route set by using parent route
pub trait SetRoute {
    /// Set the routes in fields of this struct based on the route of it's parent.
    fn set_routes_from_parent(&mut self, parent: &Route);
}

/// structs with IOs will be able to have the IOs routes set by using parent route
#[allow(clippy::upper_case_acronyms)]
pub trait SetIORoutes {
    /// Set the route and IO type of IOs in this struct based on parent's route
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

impl From<String> for Route {
    fn from(string: String) -> Self {
        Route(string)
    }
}

impl From<&String> for Route {
    fn from(string: &String) -> Self {
        Route(string.to_string())
    }
}

impl From<&Name> for Route {
    fn from(name: &Name) -> Self {
        Route(name.to_string())
    }
}

#[cfg(test)]
mod test {
    use crate::model::name::Name;
    use crate::model::validation::Validate;

    use super::Route;

    #[test]
    fn test_from_string() {
        let route = Route::from("my-route".to_string());
        assert_eq!(route, Route::from("my-route"));
    }

    #[test]
    fn test_from_ref_string() {
        let route = Route::from(&format!("{}{}", "my-route", "/subroute"));
        assert_eq!(route, Route::from("my-route/subroute"));
    }

    #[test]
    fn test_from_name() {
        let name = Name::from("my-route-name");
        assert_eq!(Route::from(&name), Route::from("my-route-name"));
    }

    #[test]
    fn test_route_pop() {
        let original = Route::from("/context/function/output/subroute");
        let (level_up, sub) = original.pop();
        assert_eq!(
            level_up.into_owned(),
            Route::from("/context/function/output")
        );
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
        assert!(!trailing_number);
    }

    #[test]
    fn just_slash_no_change() {
        let route = Route::from("/");
        let (new_route, _num, trailing_number) = route.without_trailing_array_index();
        assert_eq!(new_route.into_owned(), Route::from("/"));
        assert!(!trailing_number);
    }

    #[test]
    fn no_trailing_number_no_change() {
        let route = Route::from("/output1");
        let (new_route, _num, trailing_number) = route.without_trailing_array_index();
        assert_eq!(new_route.into_owned(), route);
        assert!(!trailing_number);
    }

    #[test]
    fn detect_array_at_output_root() {
        let route = Route::from("/0");
        let (new_route, num, trailing_number) = route.without_trailing_array_index();
        assert_eq!(new_route.into_owned(), Route::from(""));
        assert_eq!(num, 0);
        assert!(trailing_number);
    }

    #[test]
    fn detect_array_at_output_subroute() {
        let route = Route::from("/array_output/0");
        let (new_route, num, trailing_number) = route.without_trailing_array_index();
        assert_eq!(new_route.into_owned(), Route::from("/array_output"));
        assert_eq!(num, 0);
        assert!(trailing_number);
    }

    #[test]
    fn valid_process_route() {
        let route = Route::from("sub_process/i1");
        assert!(route.validate().is_ok());
    }

    #[test]
    fn valid_process_route_with_subroute() {
        let route = Route::from("sub_process/i1/sub_route");
        assert!(route.validate().is_ok());
    }

    #[test]
    fn valid_input_route() {
        let route = Route::from("input/i1");
        assert!(route.validate().is_ok());
    }

    #[test]
    fn valid_input_route_with_subroute() {
        let route = Route::from("input/i1/sub_route");
        assert!(route.validate().is_ok());
    }

    #[test]
    fn valid_output_route() {
        let route = Route::from("output/i1");
        assert!(route.validate().is_ok());
    }

    #[test]
    fn valid_output_route_with_subroute() {
        let route = Route::from("output/i1/sub_route");
        assert!(route.validate().is_ok());
    }

    #[test]
    fn validate_invalid_empty_route() {
        let route = Route::from("");
        assert!(route.validate().is_err());
    }

    #[test]
    fn validate_invalid_route() {
        let route = Route::from("123");
        assert!(route.validate().is_err());
    }

    #[test]
    fn subroute_equal_route() {
        let route = Route::from("/context/function");
        assert!(route
            .sub_route_of(&Route::from("/context/function"))
            .is_some())
    }

    #[test]
    fn subroute_distinct_route() {
        let route = Route::from("/context/function");
        assert!(route.sub_route_of(&Route::from("/context/foo")).is_none())
    }

    #[test]
    fn subroute_extended_name_route() {
        let route = Route::from("/context/function_foo");
        assert!(route
            .sub_route_of(&Route::from("/context/function"))
            .is_none())
    }

    #[test]
    fn is_a_subroute() {
        let route = Route::from("/context/function/input");
        assert!(route
            .sub_route_of(&Route::from("/context/function"))
            .is_some())
    }

    #[test]
    fn is_a_sub_subroute() {
        let route = Route::from("/context/function/input/element");
        assert!(route
            .sub_route_of(&Route::from("/context/function"))
            .is_some())
    }

    #[test]
    fn is_array_element_subroute() {
        let route = Route::from("/context/function/1");
        assert!(route
            .sub_route_of(&Route::from("/context/function"))
            .is_some())
    }

    #[test]
    fn is_array_element_sub_subroute() {
        let route = Route::from("/context/function/input/1");
        assert!(route
            .sub_route_of(&Route::from("/context/function"))
            .is_some())
    }

    #[test]
    fn extend_empty_route() {
        let mut route = Route::default();

        route.extend(&Route::from("sub"));
        assert_eq!(route, Route::from("/sub"));
    }

    #[test]
    fn extend_root_route() {
        let mut route = Route::from("/");

        route.extend(&Route::from("sub"));
        assert_eq!(route, Route::from("/sub"));
    }

    #[test]
    fn extend_route() {
        let mut route = Route::from("/context/function");

        route.extend(&Route::from("sub"));
        assert_eq!(route, Route::from("/context/function/sub"));
    }

    #[test]
    fn extend_route_with_nothing() {
        let mut route = Route::from("/context/function");

        route.extend(&Route::from(""));
        assert_eq!(route, Route::from("/context/function"));
    }
}

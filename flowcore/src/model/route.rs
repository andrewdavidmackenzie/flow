use std::borrow::Cow;
use std::fmt;
use std::marker::PhantomData;
use std::str::FromStr;
use serde::de::Visitor;
use serde::{de, Deserialize, Deserializer};

use serde_derive::Serialize;
use crate::errors;

use crate::errors::{Error, Result, bail};
use crate::model::io::IOType;
use crate::model::name::Name;
use crate::model::validation::Validate;

/// A [Route] defines a particular location within the flow hierarchy
/// and can be used to refer to a function, flow, input or output uniquely
#[derive(Hash, Debug, PartialEq, Ord, PartialOrd, Eq, Clone, Default, Serialize, Deserialize)]
pub struct Route {
    string: String,
}

impl fmt::Display for Route {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.string)
    }
}

impl FromStr for Route {
    type Err = errors::Error;

    fn from_str(s: &str) -> Result<Self> {
        Ok(Route {
            string: s.to_string()
        })
    }
}

impl From<&str> for Route {
    fn from(string: &str) -> Self {
        Route {
            string: string.to_string()
        }
    }
}

impl From<String> for Route {
    fn from(string: String) -> Self {
        Route {
            string
        }
    }
}

impl From<&Name> for Route {
    fn from(name: &Name) -> Self {
        Route {
            string: name.to_string()
        }
    }
}

/// A custom Deserializer for a String into a [Route]
///
/// # Errors
///
/// Returns `Err` if the bytes cannot be deserialized as a `Route`
#[allow(clippy::module_name_repetitions)]
pub fn route_string<'de, T, D>(deserializer: D) -> std::result::Result<T, D::Error>
    where
        T: Deserialize<'de> + FromStr<Err = Error>,
        D: Deserializer<'de>,
{
    struct RouteString<T>(PhantomData<fn() -> T>);

    impl<'de, Route> Visitor<'de> for RouteString<Route>
        where
            Route: Deserialize<'de> + FromStr<Err = Error>,
    {
        type Value = Route;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("String")
        }

        fn visit_str<E>(self, value: &str) -> std::result::Result<Route, E>
        where E: de::Error {
            #[allow(clippy::unwrap_used)]
            Ok(FromStr::from_str(value).unwrap())
        }
    }

    deserializer.deserialize_any(RouteString(PhantomData))
}

/// A custom deserializer for a String or an Array (Sequence) of Strings for Routes
///
/// # Errors
///
/// Returns `Err` data cannot be deserializes as a `Route` or Array of `Route`
#[allow(clippy::module_name_repetitions)]
pub fn route_or_route_array<'de, D>(deserializer: D) -> std::result::Result<Vec<Route>, D::Error>
    where
        D: Deserializer<'de>,
{
    struct StringOrVec(PhantomData<Vec<Route>>);

    impl<'de> de::Visitor<'de> for StringOrVec {
        type Value = Vec<Route>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("Route or Array of Routes")
        }

        fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
        where E: de::Error {
            #[allow(clippy::unwrap_used)]
            Ok(vec![FromStr::from_str(value).unwrap()])
        }

        fn visit_seq<S>(self, mut visitor: S) -> std::result::Result<Self::Value, S::Error>
        where S: de::SeqAccess<'de> {
            let mut vec: Vec<Route> = Vec::new();

            while let Some(element) = visitor.next_element::<String>()? {
                vec.push(Route::from(element));
            }

            Ok(vec)
        }
    }

    deserializer.deserialize_any(StringOrVec(PhantomData))
}

/// A [Route] can refer to a number of different types of objects in the flow hierarchy
#[derive(Debug, PartialEq)]
#[allow(clippy::module_name_repetitions)]
pub enum RouteType {
    /// The route refers to an Input of a Flow
    FlowInput(Name, Route),
    /// The Route refers to the Output of a Flow
    FlowOutput(Name),
    /// The route specifies a sub-process of a flow (i.e. a sub-flow or a function)
    SubProcess(Name, Route)
}

/// `Route` is used to locate Processes (Flows or Functions), their IOs and sub-elements of a
/// data structure within the flow hierarchy
///
/// Examples
/// "/my-flow" -> The flow called "my-flow, anchored at the root of the hierarchy, i.e. the context
/// "/my-flow/sub-flow" -> A flow called "sub-flow" that is within "my-flow"
/// "/my-flow/sub-flow/function" -> A function called "function" within "sub-flow"
/// "/my-flow/sub-flow/function/input_1" -> An IO called `input_1` of "function"
/// "/my-flow/sub-flow/function/input_1/1" -> An array element at index 1 of the Array output from `input_1`
/// "/my-flow/sub-flow/function/input_2/part_a" -> A part of the Json structure output by `input_2` called `part_a`
impl Route {
    /// `sub_route_of` returns an `Option<Route`> indicating if `self` is a subroute of `other`
    /// (i.e. `self` is a longer route to an element under the `other` route)
    /// Return values
    ///     None                    - `self` is not a sub-route of `other`
    ///     (e.g. ("/my-route1", "/my-route2")
    ///     (e.g. ("/my-route1", "/my-route1/something")
    ///     `Some(Route::from(""))`   - `self` and `other` are equal
    ///     (e.g. ("/my-route1", "/my-route1")
    ///     `Some(Route::from(diff))` - `self` is a sub-route of `other` - with `diff` added
    ///     (e.g. ("/my-route1/something", "/my-route1")
    pub fn sub_route_of(&self, other: &Route) -> Option<Route> {
        if self.string == other.string {
            return Some(Route::from(""));
        }

        self.string.strip_prefix(&format!("{other}/")).map(Route::from)
    }

    /// Insert another Route at the front of this Route
    pub fn insert<R: AsRef<str>>(&mut self, sub_route: R) -> &Self {
        self.string.insert_str(0, sub_route.as_ref());
        self
    }

    /// Extend a Route by appending another Route to the end, adding the '/' separator if needed
    pub fn extend(&mut self, sub_route: &Route) -> &Self {
        if !sub_route.is_empty() {
            if !self.string.ends_with('/') && !sub_route.string.starts_with('/') {
                self.string.push('/');
            }
            self.string.push_str(&sub_route.string);
        }

        self
    }

    /// Return the [Route] that is one level up, if it exists
    /// Example: `/context/function/output/subroute -> /context/function/output`
    #[must_use]
    pub fn pop(&self) -> (Cow<'_, Route>, Option<Route>) {
        let mut segments: Vec<&str> = self.string.split('/').collect();
        let sub_route = segments.pop();
        match sub_route {
            None | Some("") => (Cow::Borrowed(self), None),
            Some(sr) => (
                Cow::Owned(Route::from(segments.join("/"))),
                Some(Route::from(sr)),
            ),
        }
    }

    /// Return the io [Route] without a trailing number (array index) and if it has one or not
    /// If the trailing number was present then return the route with a trailing '/'
    #[must_use]
    pub fn without_trailing_array_index(&self) -> (Cow<'_, Route>, usize, bool) {
        let mut parts: Vec<&str> = self.string.split('/').collect();
        if let Some(last_part) = parts.pop() {
            if let Ok(index) = last_part.parse::<usize>() {
                let route_without_number = parts.join("/");
                return (Cow::Owned(Route::from(route_without_number)), index, true);
            }
        }

        (Cow::Borrowed(self), 0, false)
    }

    /// Return true if the [Route] selects an element from an array
    #[must_use]
    pub fn is_array_selector(&self) -> bool {
        if self.string.is_empty() {
            return false;
        }

        let mut parts: Vec<&str> = self.string.split('/').collect();
        if let Some(last_part) = parts.pop() {
            return last_part.parse::<usize>().is_ok();
        }

        false
    }

    /// Return the depth of the [Route] used to specify types and subtypes such as
    /// "" -> 0
    /// "string" -> 0
    /// "array/string" -> 1
    /// "array/array/string" -> 2
    #[must_use]
    pub fn depth(&self) -> usize {
        if self.string.is_empty() {
            return 0;
        }

        self.string.split('/').count()
    }

    /// Return true if this [Route] is empty, false otherwise
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.string.is_empty()
    }

    /// Parse the [Route] into the specific type of sub-route
    ///
    /// # Errors
    ///
    /// Returns `Err` if an invalid route has been set
    pub fn parse_subroute(&self) -> Result<RouteType> {
        let segments: Vec<&str> = self.string.split('/').collect();

        match *(segments.first().ok_or("Could not get subroute segment[0]")?) {
            "input" => {
                let name = segments.get(1).ok_or("Could not get segment[1]")?;
                let route = segments.get(2..).ok_or("Could not get segments[2..]")?
                    .join("/");
                Ok(RouteType::FlowInput((*name).to_string(),
                                        (*route).to_string().into()))
            },
            "output" => {
                let name = segments.get(1).ok_or("Could not get segment[1]")?;
                Ok(RouteType::FlowOutput((*name).to_string()))
            },
            "" => bail!("Invalid Route in connection - must be an input, output or sub-process name"),
            process_name => Ok(RouteType::SubProcess(process_name.into(),
                                                     segments.get(1..).ok_or("Could not get segments[1..]")?
                                                         .join("/").to_string().into())),
        }
    }

    /// Return the [Route] the parent of the supplied [Name]
    #[must_use]
    pub fn parent(&self, name: &Name) -> String {
        self.string.strip_suffix(&format!("/{}", name.as_str()))
            .unwrap_or(&self.string).to_string()
    }
}

impl AsRef<str> for Route {
    fn as_ref(&self) -> &str {
        self.string.as_str()
    }
}

impl Validate for Route {
    fn validate(&self) -> Result<()> {
        if self.string.is_empty() {
            bail!("Route '{}' is invalid - a route must specify an input, output or subprocess by name", self);
        }

        if self.string.parse::<usize>().is_ok() {
            bail!("Route '{}' is invalid - cannot be an integer", self);
        }

        Ok(())
    }
}

/// A trait that should be implemented by structs to indicate it has Routes
#[must_use]
#[allow(clippy::module_name_repetitions)]
pub trait HasRoute {
    /// Return a reference to the Route of the struct that implements this trait
    fn route(&self) -> &Route;
    /// Return a mutable reference to the Route of the struct that implements this trait
    fn route_mut(&mut self) -> &mut Route;
}

/// Some structs with Routes will be able to have their route set by using parent route
#[must_use]
#[allow(clippy::module_name_repetitions)]
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

#[cfg(test)]
mod test {
    use crate::model::name::Name;
    use crate::model::route::RouteType;
    use crate::model::validation::Validate;

    use super::Route;

    #[test]
    fn test_invalid_connection_route() {
        match Route::from("").parse_subroute() {
            Ok(_) => panic!("Connection route should not be valid"),
            Err(e) => assert!(e.to_string()
                .contains("Invalid Route in connection"))
        }
    }

    #[test]
    fn test_parse_valid_input() {
        let route = Route::from("input/string");
        assert_eq!(route.parse_subroute().expect("Could not find input"),
                   RouteType::FlowInput(Name::from("string"),
                                        Route::default()));
    }

    #[test]
    fn test_parse_valid_output() {
        let route = Route::from("output/string");
        assert_eq!(route.parse_subroute().expect("Could not find input"),
                   RouteType::FlowOutput(Name::from("string")));
    }

    #[test]
    fn test_parse_valid_subprocess() {
        let route = Route::from("sub-process");
        assert_eq!(route.parse_subroute().expect("Could not find input"),
                   RouteType::SubProcess(Name::from("sub-process"), Route::default()));
    }

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
        let original = Route::from("/root/function/output/subroute");
        let (level_up, sub) = original.pop();
        assert_eq!(
            level_up.into_owned(),
            Route::from("/root/function/output")
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
        let route = Route::from("/root/function");
        assert!(route
            .sub_route_of(&Route::from("/root/function"))
            .is_some());
    }

    #[test]
    fn subroute_distinct_route() {
        let route = Route::from("/root/function");
        assert!(route.sub_route_of(&Route::from("/root/foo")).is_none());
    }

    #[test]
    fn subroute_extended_name_route() {
        let route = Route::from("/root/function_foo");
        assert!(route
            .sub_route_of(&Route::from("/root/function"))
            .is_none());
    }

    #[test]
    fn is_a_subroute() {
        let route = Route::from("/root/function/input");
        assert!(route
            .sub_route_of(&Route::from("/root/function"))
            .is_some());
    }

    #[test]
    fn is_a_sub_subroute() {
        let route = Route::from("/root/function/input/element");
        assert!(route
            .sub_route_of(&Route::from("/root/function"))
            .is_some());
    }

    #[test]
    fn is_array_element_subroute() {
        let route = Route::from("/root/function/1");
        assert!(route
            .sub_route_of(&Route::from("/root/function"))
            .is_some());
    }

    #[test]
    fn is_array_element_sub_subroute() {
        let route = Route::from("/root/function/input/1");
        assert!(route
            .sub_route_of(&Route::from("/root/function"))
            .is_some());
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
        let mut route = Route::from("/root/function");

        route.extend(&Route::from("sub"));
        assert_eq!(route, Route::from("/root/function/sub"));
    }

    #[test]
    fn extend_route_with_nothing() {
        let mut route = Route::from("/root/function");

        route.extend(&Route::from(""));
        assert_eq!(route, Route::from("/root/function"));
    }
}

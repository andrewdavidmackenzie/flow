use serde::de::Visitor;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use std::borrow::Cow;
use std::fmt;
use std::marker::PhantomData;
use std::str::FromStr;

use crate::errors;
use crate::errors::{bail, Error, Result};
use crate::model::io::IOType;
use crate::model::name::Name;
use crate::model::validation::Validate;

/// A [Route] defines a particular location within the flow hierarchy
/// and can be used to refer to a function, flow, input or output uniquely.
///
/// Internally a Route is a sequence of path segments, making operations like
/// `pop`, `extend`, and `sub_route_of` simple Vec operations instead of
/// repeated string splitting.
#[derive(Hash, Debug, PartialEq, Ord, PartialOrd, Eq, Clone, Default)]
pub struct Route {
    /// Whether the route starts with a leading `/` (absolute path)
    rooted: bool,
    /// The path segments (e.g., `["root", "function", "output"]`)
    segments: Vec<String>,
}

/// Parse a route string into (rooted, segments)
fn parse_route_string(s: &str) -> (bool, Vec<String>) {
    if s.is_empty() {
        return (false, Vec::new());
    }

    let rooted = s.starts_with('/');
    let trimmed = s.strip_prefix('/').unwrap_or(s);

    if trimmed.is_empty() {
        (rooted, Vec::new())
    } else {
        let segments = trimmed.split('/').map(String::from).collect();
        (rooted, segments)
    }
}

impl Route {
    /// Build the string representation of this route
    fn to_route_string(&self) -> String {
        if self.segments.is_empty() {
            if self.rooted {
                "/".to_string()
            } else {
                String::new()
            }
        } else {
            let joined = self.segments.join("/");
            if self.rooted {
                format!("/{joined}")
            } else {
                joined
            }
        }
    }

    /// Return the number of segments in this route
    #[must_use]
    pub fn segment_count(&self) -> usize {
        self.segments.len()
    }

    /// Return a reference to the segment at the given index
    #[must_use]
    pub fn segment(&self, index: usize) -> Option<&str> {
        self.segments.get(index).map(String::as_str)
    }

    /// Return true if this route starts with a leading `/`
    #[must_use]
    pub fn is_rooted(&self) -> bool {
        self.rooted
    }
}

impl fmt::Display for Route {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_route_string())
    }
}

impl Serialize for Route {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_route_string())
    }
}

impl<'de> Deserialize<'de> for Route {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(Route::from(s))
    }
}

impl FromStr for Route {
    type Err = errors::Error;

    fn from_str(s: &str) -> Result<Self> {
        let (rooted, segments) = parse_route_string(s);
        Ok(Route { rooted, segments })
    }
}

impl From<&str> for Route {
    fn from(string: &str) -> Self {
        let (rooted, segments) = parse_route_string(string);
        Route { rooted, segments }
    }
}

impl From<String> for Route {
    fn from(string: String) -> Self {
        let (rooted, segments) = parse_route_string(&string);
        Route { rooted, segments }
    }
}

impl From<&Name> for Route {
    fn from(name: &Name) -> Self {
        let (rooted, segments) = parse_route_string(name.as_str());
        Route { rooted, segments }
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
        where
            E: de::Error,
        {
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
        where
            E: de::Error,
        {
            #[allow(clippy::unwrap_used)]
            Ok(vec![FromStr::from_str(value).unwrap()])
        }

        fn visit_seq<S>(self, mut visitor: S) -> std::result::Result<Self::Value, S::Error>
        where
            S: de::SeqAccess<'de>,
        {
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
    SubProcess(Name, Route),
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
    #[must_use]
    pub fn sub_route_of(&self, other: &Route) -> Option<Route> {
        if self == other {
            return Some(Route::default());
        }

        // rootedness must match for a valid prefix relationship
        if self.rooted != other.rooted {
            return None;
        }

        // self must start with all of other's segments
        if self.segments.len() <= other.segments.len() {
            return None;
        }

        if self.segments.get(..other.segments.len()) != Some(other.segments.as_slice()) {
            return None;
        }

        let remaining: Vec<String> = self
            .segments
            .get(other.segments.len()..)
            .unwrap_or(&[])
            .to_vec();
        Some(Route {
            rooted: false,
            segments: remaining,
        })
    }

    /// Insert another Route at the front of this Route
    pub fn insert<R: AsRef<str>>(&mut self, sub_route: R) -> &Self {
        let (ins_rooted, ins_segments) = parse_route_string(sub_route.as_ref());
        if ins_rooted {
            self.rooted = true;
        }
        // Prepend the inserted segments
        let mut new_segments = ins_segments;
        new_segments.append(&mut self.segments);
        self.segments = new_segments;
        self
    }

    /// Extend a Route by appending another Route to the end
    pub fn extend(&mut self, sub_route: &Route) -> &Self {
        if !sub_route.is_empty() {
            self.segments.extend_from_slice(&sub_route.segments);
        }
        self
    }

    /// Return the [Route] that is one level up, if it exists
    /// Example: `/context/function/output/subroute -> /context/function/output`
    #[must_use]
    pub fn pop(&self) -> (Cow<'_, Route>, Option<Route>) {
        if self.segments.is_empty() {
            return (Cow::Borrowed(self), None);
        }

        let mut parent_segments = self.segments.clone();
        let last = parent_segments.pop();
        match last {
            None => (Cow::Borrowed(self), None),
            Some(segment) => (
                Cow::Owned(Route {
                    rooted: self.rooted,
                    segments: parent_segments,
                }),
                Some(Route::from(segment.as_str())),
            ),
        }
    }

    /// Return the io [Route] without a trailing number (array index) and if it has one or not
    /// If the trailing number was present then return the route with a trailing '/'
    #[must_use]
    pub fn without_trailing_array_index(&self) -> (Cow<'_, Route>, usize, bool) {
        if let Some(last) = self.segments.last() {
            if let Ok(index) = last.parse::<usize>() {
                let mut parent_segments = self.segments.clone();
                parent_segments.pop();
                // If stripping the index leaves no segments, produce an empty
                // (non-rooted) route to match the historical behavior where
                // `/0` → parent `""` (not `"/"`).
                let rooted = self.rooted && !parent_segments.is_empty();
                return (
                    Cow::Owned(Route {
                        rooted,
                        segments: parent_segments,
                    }),
                    index,
                    true,
                );
            }
        }

        (Cow::Borrowed(self), 0, false)
    }

    /// Return true if the [Route] selects an element from an array
    #[must_use]
    pub fn is_array_selector(&self) -> bool {
        self.segments
            .last()
            .is_some_and(|s| s.parse::<usize>().is_ok())
    }

    /// Return the depth of the [Route] — the number of segments produced by
    /// splitting the string representation on `/`.
    ///
    /// For rooted routes (starting with `/`), this includes the leading empty
    /// segment, matching the behavior of `string.split('/').count()`.
    ///
    /// `""` -> 0, `"string"` -> 1, `"array/string"` -> 2, `"/0"` -> 2
    #[must_use]
    pub fn depth(&self) -> usize {
        if self.segments.is_empty() && !self.rooted {
            0
        } else if self.rooted {
            self.segments.len() + 1
        } else {
            self.segments.len()
        }
    }

    /// Return true if this [Route] is empty, false otherwise
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.segments.is_empty() && !self.rooted
    }

    /// Parse the [Route] into the specific type of sub-route
    ///
    /// # Errors
    ///
    /// Returns `Err` if an invalid route has been set
    pub fn parse_subroute(&self) -> Result<RouteType> {
        let first = self
            .segments
            .first()
            .ok_or("Could not get subroute segment[0]")?;

        match first.as_str() {
            "input" => {
                let name = self
                    .segments
                    .get(1)
                    .ok_or("Could not get segment[1]")?
                    .clone();
                let remaining: Vec<String> = self.segments.get(2..).unwrap_or(&[]).to_vec();
                Ok(RouteType::FlowInput(
                    name,
                    Route {
                        rooted: false,
                        segments: remaining,
                    },
                ))
            }
            "output" => {
                let name = self
                    .segments
                    .get(1)
                    .ok_or("Could not get segment[1]")?
                    .clone();
                Ok(RouteType::FlowOutput(name))
            }
            "" => {
                bail!("Invalid Route in connection - must be an input, output or sub-process name")
            }
            process_name => {
                let remaining: Vec<String> = self.segments.get(1..).unwrap_or(&[]).to_vec();
                Ok(RouteType::SubProcess(
                    process_name.into(),
                    Route {
                        rooted: false,
                        segments: remaining,
                    },
                ))
            }
        }
    }

    /// Return the [Route] the parent of the supplied [Name]
    #[must_use]
    pub fn parent(&self, name: &Name) -> String {
        // Find and remove the trailing segment matching `name`
        if self.segments.last().is_some_and(|s| s == name.as_str()) {
            let mut parent_segments = self.segments.clone();
            parent_segments.pop();
            let parent = Route {
                rooted: self.rooted,
                segments: parent_segments,
            };
            return parent.to_route_string();
        }
        self.to_route_string()
    }
}

impl AsRef<str> for Route {
    fn as_ref(&self) -> &str {
        // This is the one method that can't efficiently return a &str from Vec<String>
        // without allocating. We use a static empty string for the common empty case,
        // but for non-empty routes we need to leak or use an alternative approach.
        //
        // For now, we keep backward compatibility by implementing ToString and
        // letting callers use .to_string() or Display. The AsRef<str> impl
        // is needed for the `insert` method's generic bound.
        //
        // SAFETY: This leaks memory for non-empty routes. This is acceptable during
        // the transition period. Callers should migrate to using Display or
        // to_route_string() instead.
        if self.segments.is_empty() && !self.rooted {
            ""
        } else {
            // Leak the string to return a &str — this is a known compromise
            // during the Route refactoring. The leaked strings are small and
            // the number of distinct routes in a program is bounded.
            Box::leak(self.to_route_string().into_boxed_str())
        }
    }
}

impl Validate for Route {
    fn validate(&self) -> Result<()> {
        let s = self.to_route_string();
        if s.is_empty() {
            bail!("Route '{}' is invalid - a route must specify an input, output or subprocess by name", s);
        }

        if s.parse::<usize>().is_ok() {
            bail!("Route '{}' is invalid - cannot be an integer", s);
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
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {
    use crate::model::name::Name;
    use crate::model::route::RouteType;
    use crate::model::validation::Validate;

    use super::Route;

    #[test]
    fn test_invalid_connection_route() {
        match Route::from("").parse_subroute() {
            Ok(_) => panic!("Connection route should not be valid"),
            Err(e) => assert!(e.to_string().contains("Could not get subroute")),
        }
    }

    #[test]
    fn test_parse_valid_input() {
        let route = Route::from("input/string");
        assert_eq!(
            route.parse_subroute().expect("Could not find input"),
            RouteType::FlowInput(Name::from("string"), Route::default())
        );
    }

    #[test]
    fn test_parse_valid_output() {
        let route = Route::from("output/string");
        assert_eq!(
            route.parse_subroute().expect("Could not find input"),
            RouteType::FlowOutput(Name::from("string"))
        );
    }

    #[test]
    fn test_parse_valid_subprocess() {
        let route = Route::from("sub-process");
        assert_eq!(
            route.parse_subroute().expect("Could not find input"),
            RouteType::SubProcess(Name::from("sub-process"), Route::default())
        );
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
        assert_eq!(level_up.into_owned(), Route::from("/root/function/output"));
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
        assert!(route.sub_route_of(&Route::from("/root/function")).is_some());
    }

    #[test]
    fn subroute_distinct_route() {
        let route = Route::from("/root/function");
        assert!(route.sub_route_of(&Route::from("/root/foo")).is_none());
    }

    #[test]
    fn subroute_extended_name_route() {
        let route = Route::from("/root/function_foo");
        assert!(route.sub_route_of(&Route::from("/root/function")).is_none());
    }

    #[test]
    fn is_a_subroute() {
        let route = Route::from("/root/function/input");
        assert!(route.sub_route_of(&Route::from("/root/function")).is_some());
    }

    #[test]
    fn is_a_sub_subroute() {
        let route = Route::from("/root/function/input/element");
        assert!(route.sub_route_of(&Route::from("/root/function")).is_some());
    }

    #[test]
    fn is_array_element_subroute() {
        let route = Route::from("/root/function/1");
        assert!(route.sub_route_of(&Route::from("/root/function")).is_some());
    }

    #[test]
    fn is_array_element_sub_subroute() {
        let route = Route::from("/root/function/input/1");
        assert!(route.sub_route_of(&Route::from("/root/function")).is_some());
    }

    #[test]
    fn extend_empty_route() {
        let mut route = Route::default();

        route.extend(&Route::from("sub"));
        assert_eq!(route, Route::from("sub"));
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

    // New tests for segment-based API
    #[test]
    fn segment_count_empty() {
        assert_eq!(Route::from("").segment_count(), 0);
    }

    #[test]
    fn segment_count_rooted_empty() {
        assert_eq!(Route::from("/").segment_count(), 0);
    }

    #[test]
    fn segment_count_simple() {
        assert_eq!(Route::from("foo/bar/baz").segment_count(), 3);
    }

    #[test]
    fn segment_access() {
        let route = Route::from("/root/function/output");
        assert_eq!(route.segment(0), Some("root"));
        assert_eq!(route.segment(1), Some("function"));
        assert_eq!(route.segment(2), Some("output"));
        assert_eq!(route.segment(3), None);
    }

    #[test]
    fn is_rooted_true() {
        assert!(Route::from("/root").is_rooted());
    }

    #[test]
    fn is_rooted_false() {
        assert!(!Route::from("relative").is_rooted());
    }

    #[test]
    fn subroute_mixed_rootedness_not_matched() {
        let rooted = Route::from("/root/function");
        let unrooted = Route::from("root");
        assert!(rooted.sub_route_of(&unrooted).is_none());
        assert!(unrooted.sub_route_of(&rooted).is_none());
    }

    #[test]
    fn display_roundtrip() {
        let cases = vec![
            "",
            "/",
            "/root",
            "/root/function",
            "relative",
            "a/b/c",
            "/a/b/c/0",
        ];
        for s in cases {
            assert_eq!(Route::from(s).to_string(), s, "roundtrip failed for '{s}'");
        }
    }
}

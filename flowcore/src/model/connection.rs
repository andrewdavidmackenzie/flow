use std::fmt;

use log::debug;
use serde_derive::{Deserialize, Serialize};

use crate::errors::*;
use crate::model::datatype::DataType;
use crate::model::io::IO;
use crate::model::name::Name;
use crate::model::route::HasRoute;
use crate::model::route::Route;
use crate::model::validation::Validate;

/// `Connection` defines a connection between the output of one function or flow to the input
/// of another function or flow and maybe optionally named for legibility/debugging.
#[derive(Deserialize, Serialize, Default, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct Connection {
    /// Optional name given to a connection for legibility and debugging
    #[serde(default, skip_serializing_if = "String::is_empty")]
    name: Name,
    /// `from` defines the origin of the connection
    from: Route,
    /// `to` defines the destination(s) of this connection
    #[serde(deserialize_with = "super::route_array_serde::route_or_route_array")]
    to: Vec<Route>,
    /// `from_io` is used during the compilation process and refers to a found output for the connection
    // TODO make these references, not clones
    #[serde(skip)]
    from_io: IO,
    /// `to_io` is used during the compilation process and refers to a found input for the connection
    #[serde(skip)]
    to_io: IO,
    /// `level` defines at what level in the flow hierarchy this connections belongs. It is used
    /// when collapsing connections to reduce work and avoid infinite recursion
    #[serde(skip)]
    level: usize,
    /// Track the id of the flow where this connection originated
    #[serde(skip)]
    origin_flow_id: usize,
}

/// `Direction` defines whether a `Connection` is coming from an IO or to an IO
#[derive(Debug)]
#[allow(clippy::upper_case_acronyms)]
pub enum Direction {
    /// The `Connection` is `FROM` this IO to another
    FROM,
    /// The `Connection` is `TO` this IO from another
    TO,
}

impl fmt::Display for Connection {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match (self.from_io.flow_io(), self.to_io.flow_io()) {
            (true, true) => write!(
                f,
                "(f){} --> {}(f)",
                self.from_io.route(),
                self.to_io.route()
            ),
            (true, false) => write!(f, "(f){} --> {}", self.from_io.route(), self.to_io.route()),
            (false, true) => write!(
                f,
                "   {} --> {}(f)",
                self.from_io.route(),
                self.to_io.route()
            ),
            (false, false) => write!(f, "   {} --> {}", self.from_io.route(), self.to_io.route()),
        }
    }
}

impl Validate for Connection {
    // TODO maybe validate some of the combinations here that are checked in build_connections here?
    fn validate(&self) -> Result<()> {
        self.name.validate()?;
        self.from.validate()?;
        for destination in &self.to {
            destination.validate()?;
        }
        Ok(())
    }
}

impl Connection {
    /// Create a new Route with `from_route` as the source `Route` and `to_route` as the destination
    pub fn new<R>(from_route: R, to_route: R) -> Self
    where
        R: Into<Route>,
    {
        Connection {
            from: from_route.into(),
            to: vec![to_route.into()],
            ..Default::default()
        }
    }

    /// Return the name
    #[cfg(feature = "debugger")]
    pub fn name(&self) -> &Name {
        &self.name
    }

    /// Connect the `from_io` to the `to_io` inside a flow at level `level`, if they are compatible
    pub fn connect(&mut self, from_io: IO, to_io: IO, level: usize) -> Result<()> {
        // are we selecting from a sub-route of an IO, such as an array index or element of output object?
        // TODO this requires the accumulation of the subroute to be done during connection building #1192
        let from_io_subroute = "";
        if DataType::compatible_types(from_io.datatypes(), to_io.datatypes(), &Route::from(from_io_subroute)) {
            debug!(
                "Connection built from '{}' to '{}'",
                from_io.route(),
                to_io.route()
            );
            self.from_io = from_io;
            self.to_io = to_io;
            self.level = level;
            return Ok(());
        }

        bail!("Incompatible source and destination types:\nSource:      '{}/{}' of types {:?}\nDestination: '{}' of types {:?}",
            from_io.route(), from_io_subroute, from_io.datatypes(),
            to_io.route(), to_io.datatypes())
    }

    /// Return the `from` Route specified in this connection
    pub fn from(&self) -> &Route {
        &self.from
    }

    /// Return a reference to the from_io
    pub fn from_io(&self) -> &IO {
        &self.from_io
    }

    /// Return a mutable reference to the from_io
    pub fn from_io_mut(&mut self) -> &mut IO {
        &mut self.from_io
    }

    /// Return the `to` Route specified in this connection
    pub fn to(&self) -> &Vec<Route> {
        &self.to
    }

    /// Return a reference to the to_io
    pub fn to_io(&self) -> &IO {
        &self.to_io
    }

    /// Return a mutable reference to the to_io
    pub fn to_io_mut(&mut self) -> &mut IO {
        &mut self.to_io
    }

    /// Set the flow id where this connection originated
    pub fn set_origin_flow_id(&mut self, origin_flow_id: usize) {
        self.origin_flow_id = origin_flow_id;
    }

    /// Get at what level in the flow hierarchy this connection exists (source)
    pub fn level(&self) -> usize {
        self.level
    }

}

#[cfg(test)]
mod test {
    use url::Url;

    use crate::deserializers::deserializer::get_deserializer;
    use crate::errors::*;
    use crate::model::validation::Validate;

    use super::Connection;

    fn toml_from_str(content: &str) -> Result<Connection> {
        let url = Url::parse("file:///fake.toml").expect("Could not parse URL");
        let deserializer =
            get_deserializer::<Connection>(&url).expect("Could not get deserializer");
        deserializer.deserialize(content, Some(&url))
    }

    #[test]
    fn single_destination_deserialization() {
        let input_str = "
        from = 'source'
        to = 'destination'
        ";

        let connection: Result<Connection> = toml_from_str(input_str);
        assert!(connection.is_ok(), "Could not deserialize Connection");
    }

    #[test]
    fn multiple_destination_deserialization() {
        let input_str = "
        from = 'source'
        to = ['destination', 'destination2']
        ";

        let connection: Result<Connection> = toml_from_str(input_str);
        assert!(connection.is_ok(), "Could not deserialize Connection");
    }

    #[test]
    fn display_connection() {
        let connection1 = Connection::new("input/number", "process_1");
        println!("Connection: {}", connection1);
    }

    #[test]
    fn validate_connection() {
        let connection1 = Connection::new("input/number", "process_1");
        assert!(connection1.validate().is_ok());
    }

    #[test]
    fn deserialize_extra_field_fails() {
        let input_str = "
        name = 'input'
        foo = 'extra token'
        type = 'object'
        ";

        let connection: Result<Connection> = toml_from_str(input_str);
        assert!(
            connection.is_err(),
            "Deserialized invalid connection TOML without error, but should not have."
        );
    }
}

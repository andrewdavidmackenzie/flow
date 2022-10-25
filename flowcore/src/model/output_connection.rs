use std::fmt;

use serde_derive::{Deserialize, Serialize};

/// The `Conversion` enum defines what type of run-time conversion of types is to be done
#[derive(Deserialize, Serialize, Clone, PartialEq, Eq, Debug)]
pub enum Conversion {
    /// Take value and send it wrapped in an array
    WrapAsArray,
    /// Serialize an Array, sending each element as a separate value
    ArraySerialize,
}

/// This specifies the `Source` of an `OutputConnection` which can either be:
#[derive(Deserialize, Serialize, Clone, PartialEq, Eq, Debug, Hash)]
pub enum Source {
    /// A subroute of an output of a function - used as JSON pointer to select part of the output
    Output(String),
    /// A copy of the input value used to calculate the job who's output is being forwarded
    Input(usize),
}

/// `OutputConnection` contains information about a function's output connection to another function
#[derive(Deserialize, Serialize, Clone, PartialEq, Eq, Debug)]
pub struct OutputConnection {
    /// Source of the value that should be forwarded
    #[serde(default = "Source::default", skip_serializing_if = "is_default_source")]
    pub source: Source,
    /// id of the destination function of this `OutputConnection`
    pub destination_id: usize,
    /// `io_number` is the IO number the connection is connected to on the destination function
    pub destination_io_number: usize,
    /// `flow_id` is the flow_id of the target function
    pub destination_flow_id: usize,
    /// `destination` is the full route to the destination input
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub destination: String,
    /// Optional `name` the output connection can be given to aid debugging
    #[cfg(feature = "debugger")]
    #[serde(default, skip_serializing_if = "String::is_empty")]
    name: String,
}

/// If the Source is an Output and the String for the subroute is empty then we can just
/// skip serializing it
fn is_default_source(source: &Source) -> bool {
    matches!(source, Source::Output(subroute) if subroute.is_empty())
}

impl Default for Source {
    fn default() -> Self {
        Self::Output("".into())
    }
}

impl OutputConnection {
    /// Create a new `OutputConnection`
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        source: Source,
        destination_id: usize,
        destination_io_number: usize,
        flow_id: usize,
        destination: String,
        #[cfg(feature = "debugger")] name: String,
    ) -> Self {
        OutputConnection {
            source,
            destination_id,
            destination_io_number,
            destination_flow_id: flow_id,
            destination,
            #[cfg(feature = "debugger")]
            name,
        }
    }
}

impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Source::Output(subroute) if subroute.is_empty() => Ok(()),
            Source::Output(subroute) => write!(f, "{subroute}"),
            Source::Input(index) => write!(f, ":{index}"),
        }
    }
}

impl fmt::Display for OutputConnection {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Output '{}' -> Function #{}({}):{}", self.source,
               self.destination_id, self.destination_flow_id, self.destination_io_number)?;
        if !self.destination.is_empty() {
            write!(f, " @ '{}'", self.destination)?;
        }

        write!(f, "")
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn display_with_route_test() {
        let connection = super::OutputConnection::new(
            super::Source::Output("/".into()),
            1,
            1,
            1,
            "/flow1/input".into(),
            #[cfg(feature = "debugger")]
            "test-connection".into(),
        );
        println!("Connection: {connection}");
    }
}

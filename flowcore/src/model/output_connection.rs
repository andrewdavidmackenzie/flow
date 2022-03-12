use std::fmt;

use serde_derive::{Deserialize, Serialize};

/// The `Conversion` enum defines what type of run-time conversion of types is to be done
#[derive(Deserialize, Serialize, Clone, PartialEq, Debug)]
pub enum Conversion {
    /// Take value and send it wrapped in an array
    WrapAsArray,
    /// Serialize an Array, sending each element as a separate value
    ArraySerialize,
}

/// This specifies the `Source` of an `OutputConnection` which can either be:
#[derive(Deserialize, Serialize, Clone, PartialEq, Debug, Hash)]
pub enum Source {
    /// A subroute of an output of a function - used as JSON pointer to select part of the output
    Output(String),
    /// A copy of the input value used to calculate the job who's output is being forwarded
    Input(usize),
}

/// `OutputConnection` contains information about a function's output connection to another function
#[derive(Deserialize, Serialize, Clone, PartialEq, Debug)]
pub struct OutputConnection {
    /// Source of the value that should be forwarded
    #[serde(default = "Source::default", skip_serializing_if = "is_default_source")]
    pub source: Source,
    /// `function_id` is the id of the destination function of this `OutputConnection`
    pub function_id: usize,
    /// `io_number` is the IO number the connection is connected to on the destination function
    pub io_number: usize,
    /// `flow_id` is the flow_id of the target function
    pub flow_id: usize,
    /// `array_order` defines how many levels of arrays of non-array values does the destination accept
    #[serde(
        default = "default_array_order",
        skip_serializing_if = "is_default_array_order"
    )]
    pub destination_array_order: i32,
    /// `generic` defines if the input accepts generic "object"s
    #[serde(default = "default_generic", skip_serializing_if = "is_not_generic")]
    pub generic: bool,
    /// `destination` is the full route to the destination input
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub destination: String,
    /// Optional `name` the output connection can be given to aid debugging
    #[cfg(feature = "debugger")]
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
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

fn default_array_order() -> i32 {
    0
}

#[allow(clippy::trivially_copy_pass_by_ref)] // As this is imposed on us by serde
fn is_default_array_order(order: &i32) -> bool {
    *order == 0
}

fn default_generic() -> bool {
    false
}

#[allow(clippy::trivially_copy_pass_by_ref)] // As this is imposed on us by serde
fn is_not_generic(generic: &bool) -> bool {
    !*generic
}

impl OutputConnection {
    /// Create a new `OutputConnection`
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        source: Source,
        function_id: usize,
        io_number: usize,
        flow_id: usize,
        array_level_serde: i32,
        generic: bool,
        route: String,
        #[cfg(feature = "debugger")] name: String,
    ) -> Self {
        OutputConnection {
            source,
            function_id,
            io_number,
            flow_id,
            destination_array_order: array_level_serde,
            generic,
            destination: route,
            #[cfg(feature = "debugger")]
            name,
        }
    }

    /// Does the destination IO accept generic "object" types
    pub fn is_generic(&self) -> bool {
        self.generic
    }
}

impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Source::Output(subroute) if subroute.is_empty() => Ok(()),
            Source::Output(subroute) => write!(f, "Output{}", subroute),
            Source::Input(index) => write!(f, "Input #{}", index),
        }
    }
}

impl fmt::Display for OutputConnection {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Output Connection:")?;
        write!(f, "'{}'", self.source)?;
        write!(
            f,
            " -> Function #{}({}):{}",
            self.function_id, self.flow_id, self.io_number
        )?;
        if !self.destination.is_empty() {
            write!(f, " @ '{}'", self.destination)?;
        }

        write!(f, "")
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn default_array_order_test() {
        assert_eq!(super::default_array_order(), 0)
    }

    #[test]
    fn is_default_array_order_test() {
        assert!(super::is_default_array_order(&0));
    }

    #[test]
    fn is_not_default_array_order_test() {
        assert!(!super::is_default_array_order(&1));
    }

    #[test]
    fn default_generic_test() {
        assert!(!super::default_generic());
    }

    #[test]
    fn default_not_generic_test() {
        assert!(super::is_not_generic(&false));
    }

    #[test]
    fn display_test() {
        let connection = super::OutputConnection::new(
            super::Source::Output("/".into()),
            1,
            1,
            1,
            0,
            false,
            String::default(),
            #[cfg(feature = "debugger")]
            "test-connection".into(),
        );
        println!("Connection: {}", connection);
    }

    #[test]
    fn display_with_route_test() {
        let connection = super::OutputConnection::new(
            super::Source::Output("/".into()),
            1,
            1,
            1,
            0,
            false,
            "/flow1/input".into(),
            #[cfg(feature = "debugger")]
            "test-connection".into(),
        );
        println!("Connection: {}", connection);
    }
}

use std::fmt;

use serde_derive::{Deserialize, Serialize};

/// The `Conversion` enum defines what type of run-time conversaion of types is to be done
#[derive(Deserialize, Serialize, Clone, PartialEq, Debug)]
pub enum Conversion {
    WrapAsArray,
    // Take value and send it wrapped in an array
    ArraySerialize,  // Serialize an Array, sending each element as a separate value
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Debug)]
/// `OutputConnection` contains information about a function's output connection to another function
pub struct OutputConnection {
    /// `subpath` is the path of the output of a function - used as JSON pointer to select part of the output
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub subpath: String,
    /// `function_id` is the id of the destination function of this `OutputConnection`
    pub function_id: usize,
    /// `io_number` is the IO number the connection is connected to on the destination function
    pub io_number: usize,
    /// `flow_id` is the flow_id of the target function
    pub flow_id: usize,
    /// `conversion` determines what type conversion should be done when sending to this destination
    #[serde(default = "default_conversion", skip_serializing_if = "Option::is_none")]
    pub conversion: Option<Conversion>,
    /// `route` is the full route to the destination input
    #[serde(default = "default_destination_route", skip_serializing_if = "Option::is_none")]
    pub route: Option<String>,
}

impl OutputConnection {
    /// Create a new `OutputConnection`
    pub fn new(subpath: String,
               function_id: usize,
               io_number: usize,
               flow_id: usize,
               conversion: Option<Conversion>,
               route: Option<String>, ) -> Self {
        OutputConnection {
            subpath,
            function_id,
            io_number,
            flow_id,
            conversion,
            route,
        }
    }
}

fn default_conversion() -> Option<Conversion> {
    None
}

fn default_destination_route() -> Option<String> {
    None
}

impl fmt::Display for OutputConnection {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Output Connection")?;
        if !self.subpath.is_empty() {
            write!(f, " from sub-path '{}'", self.subpath)?;
        }
        write!(f, " -> Function #{}({}):{}", self.function_id, self.flow_id, self.io_number)?;
        if let Some(route) = &self.route {
            write!(f, " @ route '{}'", route)?;
        }

        write!(f, "")
    }
}
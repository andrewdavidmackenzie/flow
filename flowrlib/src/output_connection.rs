use std::fmt;

use serde_derive::{Deserialize, Serialize};

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
    /// `route` is the full route to the destination input
    #[serde(default = "default_destination_route", skip_serializing_if = "Option::is_none")]
    pub route: Option<String>,
}

impl OutputConnection {
    /// Create a new `OutputConnection`
    pub fn new(output_subpath: String,
               destination_function_id: usize,
               destination_io_number: usize,
               destination_flow_id: usize,
               destination_route: Option<String>, ) -> Self {
        OutputConnection {
            subpath: output_subpath,
            function_id: destination_function_id,
            io_number: destination_io_number,
            flow_id: destination_flow_id,
            route: destination_route,
        }
    }
}

fn default_destination_route() -> Option<String> {
    None
}

impl fmt::Display for OutputConnection {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Output Connection")?;
        if !self.subpath.is_empty() {
            write!(f, " from sub-path '/{}'", self.subpath)?;
        }
        write!(f, " -> Function #{} Input :{}", self.function_id, self.io_number)?;
        if let Some(route) = &self.route {
            write!(f, " @ route '{}'", route)?;
        }

        write!(f, "")
    }
}
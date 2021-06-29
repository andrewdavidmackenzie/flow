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

#[derive(Deserialize, Serialize, Clone, PartialEq, Debug)]
/// `OutputConnection` contains information about a function's output connection to another function
pub struct OutputConnection {
    /// `subroute` is the path of the output of a function - used as JSON pointer to select part of the output
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub subroute: String,
    /// `function_id` is the id of the destination function of this `OutputConnection`
    pub function_id: usize,
    /// `io_number` is the IO number the connection is connected to on the destination function
    pub io_number: usize,
    /// `flow_id` is the flow_id of the target function
    pub flow_id: usize,
    /// `array_order` defines how many levels of arrays of non-array values does the destination accept
    #[serde(
        default = "default_array_level_serde",
        skip_serializing_if = "is_default_array_level_serde"
    )]
    pub array_level_serde: i32,
    /// `generic` defines if the input accepts generic "Value"s
    #[serde(default = "default_generic", skip_serializing_if = "is_not_generic")]
    pub generic: bool,
    /// `route` is the full route to the destination input
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub route: String,
    /// Optional `name` the output connection can be given to aid debugging
    #[cfg(feature = "debugger")]
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
}

fn default_array_level_serde() -> i32 {
    0
}

#[allow(clippy::trivially_copy_pass_by_ref)] // As this is imposed on us by serde
fn is_default_array_level_serde(order: &i32) -> bool {
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
        subroute: String,
        function_id: usize,
        io_number: usize,
        flow_id: usize,
        array_level_serde: i32,
        generic: bool,
        route: String,
        #[cfg(feature = "debugger")] name: String,
    ) -> Self {
        OutputConnection {
            subroute,
            function_id,
            io_number,
            flow_id,
            array_level_serde,
            generic,
            route,
            #[cfg(feature = "debugger")]
            name,
        }
    }

    /// Does the destination IO accept generic "Value" types
    pub fn is_generic(&self) -> bool {
        self.generic
    }
}

impl fmt::Display for OutputConnection {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Output Connection:")?;
        if !self.subroute.is_empty() {
            write!(f, " '{}'", self.subroute)?;
        }
        write!(
            f,
            " -> Function #{}({}):{}",
            self.function_id, self.flow_id, self.io_number
        )?;
        if !self.route.is_empty() {
            write!(f, " @ '{}'", self.route)?;
        }

        write!(f, "")
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn default_array_order_test() {
        assert_eq!(super::default_array_level_serde(), 0)
    }

    #[test]
    fn is_default_array_order_test() {
        assert!(super::is_default_array_level_serde(&0));
    }

    #[test]
    fn is_not_default_array_order_test() {
        assert!(!super::is_default_array_level_serde(&1));
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
            "/".into(),
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
            "/".into(),
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

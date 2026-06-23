#![deny(clippy::unwrap_used, clippy::expect_used)]
//! `flowcore` defines core structs and traits used by other flow libraries and implementations

use serde_json::Value;

use crate::errors::Result;

/// Return a flow function result with `Ok((Some(value), RUN_AGAIN))`.
///
/// Single value: `flow_output!(json!(42))`
/// Named outputs: `flow_output!("result" => json!(42), "remainder" => json!(0))`
///
/// # Examples
/// ```
/// use flowcore::flow_output;
/// use serde_json::json;
///
/// // Single value output
/// let result: flowcore::errors::Result<(Option<serde_json::Value>, bool)> =
///     flow_output!(json!(42));
/// assert_eq!(result.unwrap().0, Some(json!(42)));
///
/// // Named output map
/// let result: flowcore::errors::Result<(Option<serde_json::Value>, bool)> =
///     flow_output!("result" => json!(42), "remainder" => json!(0));
/// assert_eq!(result.unwrap().0.unwrap().pointer("/result").unwrap(), &json!(42));
/// ```
#[macro_export]
macro_rules! flow_output {
    // Single value output
    ($val:expr) => {
        Ok((Some($val), $crate::RUN_AGAIN))
    };
    // Named output map
    ($($key:expr => $val:expr),+ $(,)?) => {{
        let mut map = serde_json::Map::new();
        $(map.insert($key.into(), $val);)+
        Ok((Some(serde_json::Value::Object(map)), $crate::RUN_AGAIN))
    }};
}

/// serializers to read definition files from various text formats based on file extension
pub mod deserializers;

/// Serializers for writing flow model types to disk (TOML format)
pub mod serializers;

/// contains [Error] that other modules in this crate will `use errors::*;`
/// to get access to everything `error_chain` creates.
pub mod errors;

/// `meta_provider` resolves library references of the type "lib://" and "context://"
#[cfg(all(not(target_arch = "wasm32"), feature = "meta_provider"))]
pub mod meta_provider;

/// defines many of the core data structures used across libraries and binaries
pub mod model;

/// is a trait definition that providers of content must implement
pub mod provider;

/// Utility functions related to [Urls][url::Url]
pub mod url_helper;

/// Provides well-known service names used across multiple binary crates
pub mod services;

/// Optional mDNS-SD service discovery helpers
#[cfg(not(target_arch = "wasm32"))]
pub mod discovery;

/// Return a JSON integer when the float value is a whole number, otherwise a float.
///
/// Use this in function implementations to avoid unnecessary `.0` precision
/// in numeric outputs (e.g., `sqrt(81)` returns `9` instead of `9.0`).
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::float_cmp
)]
#[must_use]
pub fn numeric_json(f: f64) -> Value {
    if f.fract() == 0.0 && f.abs() < i64::MAX as f64 {
        let i = f as i64;
        if (i as f64) == f {
            return serde_json::json!(i);
        }
    }
    serde_json::json!(f)
}

/// Use `DONT_RUN_AGAIN` to indicate that a function should not be executed more times
pub const DONT_RUN_AGAIN: RunAgain = false;

/// Use `RUN_AGAIN` to indicate that a function can be executed more times
pub const RUN_AGAIN: RunAgain = true;

/// Implementations should return a value of type `RunAgain` to indicate if it should be
/// executed more times in the future.
pub type RunAgain = bool;

/// Graph layout constants and algorithms shared between flowc and flowedit
pub mod graph;

/// contains the file and http content provider implementations
#[cfg(not(target_arch = "wasm32"))]
pub mod content;

/// Platform-standard directory paths for flow data
#[cfg(not(target_arch = "wasm32"))]
pub mod dirs;

/// The `Implementation` trait used by functions to provide the code that runs on inputs
///
/// A function's implementation must implement this trait with a single `run()` method that takes
/// as input an array of values, and it returns a `Result` tuple with an Optional output `Value`
/// plus a `RunAgain` indicating if it should be run again.
/// i.e. it has not "completed", in which case it should not be called again.
///
/// # Examples
///
/// Here is an example implementation of this trait:
///
/// ```
/// use flowcore::{Implementation, RUN_AGAIN, RunAgain};
/// use flowcore::errors::Result;
/// use serde_json::Value;
/// use serde_json::json;
///
/// #[derive(Debug)]
/// pub struct Compare;
///
/// /*
///     A compare implementation that takes two numbers and outputs the comparisons between them
/// */
/// impl Implementation for Compare {
///     fn run(&self, mut inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
///         let left = inputs[0].as_i64().unwrap();
///         let right = inputs[1].as_i64().unwrap();
///
///         let output = json!({
///                     "equal" : left == right,
///                     "lt" : left < right,
///                     "gt" : left > right,
///                     "lte" : left <= right,
///                     "gte" : left >= right
///                 });
///
///         Ok((None, RUN_AGAIN))
///     }
/// }
/// ```
///
/// **Note**: It is recommended to implement this trait by using the `flow_function` macro from the
/// `flowmacro` crate to simplify input gathering and to hide the boilerplate code around the
/// function implementing the logic.
pub trait Implementation: Sync + Send {
    /// The `run` method is used to execute the function's implementation
    ///
    /// # Errors
    ///
    /// Returns an error if the implementation detects an error loading the input values or
    /// executing the function required
    ///
    fn run(&self, inputs: &[Value]) -> Result<(Option<Value>, RunAgain)>;
}

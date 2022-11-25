#![deny(missing_docs)]
#![warn(clippy::unwrap_used)]

//! `flowcore` defines core structs and traits used by other flow libraries and implementations

use serde_json::Value;

use crate::errors::*;

/// a set of serializers to read definition files from various text formats based on file extension
pub mod deserializers;

/// contains [errors::Error] that other modules in this crate will `use errors::*;`
/// to get access to everything `error_chain` creates.
pub mod errors;

/// is used to resolve library references of the type "lib://" and "context://" using lib search path
#[cfg(all(not(target_arch = "wasm32"), feature = "meta_provider"))]
pub mod meta_provider;

/// defines many of the core data structures used across libraries and binaries
pub mod model;

/// is a trait definition that providers of content must implement
pub mod provider;

/// Utility functions related to [Urls][url::Url]
pub mod url_helper;

/// Use `DONT_RUN_AGAIN` to indicate that a function should not be executed more times
pub const DONT_RUN_AGAIN: RunAgain = false;

/// Use `RUN_AGAIN` to indicate that a function can be executed more times
pub const RUN_AGAIN: RunAgain = true;

/// Implementations should return a value of type `RunAgain` to indicate if it should be
/// executed more times in the future.
pub type RunAgain = bool;

/// contains the file and http content provider implementations
#[cfg(not(target_arch = "wasm32"))]
mod content;

/// A function's implementation must implement this trait with a single `run()` method that takes
/// as input an array of values and it returns a `Result` tuple with an Optional output `Value`
/// plus a [RunAgain] indicating if it should be run again.
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
/// `flowmacro` crate to simplify input gathering and to hide the boiler plate code around the
/// function implementing the logic.
pub trait Implementation: Sync + Send {
    /// The `run` method is used to execute the function's implementation
    fn run(&self, inputs: &[Value]) -> Result<(Option<Value>, RunAgain)>;
}

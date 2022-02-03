#![deny(missing_docs)]
#![warn(clippy::unwrap_used)]

//! `flowcore` defines core structs and traits used by other flow libraries and implementations

use serde_json::Value;

/// `content` module contains the content providers for files and http/https
#[cfg(feature = "code")]
mod content;

/// A set of serializers to read flow models from various text formats based on file extension
#[cfg(feature = "code")]
pub mod deserializers;

/// We'll put our errors in an `errors` module, and other modules in this crate will `use errors::*;`
/// to get access to everything `error_chain` creates.
#[cfg(feature = "code")]
pub mod errors;

/// `flow_manifest` is the struct that specifies the manifest of functions in a flow
#[cfg(feature = "code")]
pub mod flow_manifest;

/// `function` defines functions that form part of a flow
#[cfg(feature = "code")]
pub mod runtime_function;

/// `input` defines the struct for inputs to functions in a flow
#[cfg(feature = "code")]
pub mod input;

/// `lib_manifest` defines the structs for specifying a Library's manifest and methods to load it
#[cfg(feature = "code")]
pub mod lib_manifest;

/// `lib_provider` is used to resolve library references of the type "lib://" using lib search path
#[cfg(feature = "code")]
pub mod lib_provider;

/// `output_connection` defines a struct for a function's output connection
#[cfg(feature = "code")]
pub mod output_connection;

/// Utility functions related to Urls
#[cfg(feature = "code")]
pub mod url_helper;

/// Implementations should return a value of type `RunAgain` to indicate if it should be
/// executed more times in the future.
pub type RunAgain = bool;
/// Use `RUN_AGAIN` to indicate that a function can be executed more times
pub const RUN_AGAIN: RunAgain = true;
/// Use `DONT_RUN_AGAIN` to indicate that a function should not be executed more times
pub const DONT_RUN_AGAIN: RunAgain = false;

/// An implementation runs with an array of inputs and returns a value (or null) and a
/// bool indicating if it should be ran again.
///
/// Any 'implementation' of a function must implement this trait
///
/// # Examples
///
/// Here is an example implementation of this trait:
///
/// ```
/// use flowcore::{Implementation, RUN_AGAIN, RunAgain};
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
///     fn run(&self, mut inputs: &[Value]) -> (Option<Value>, RunAgain) {
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
///         (None, RUN_AGAIN)
///     }
/// }
/// ```
pub trait Implementation: Sync + Send {
    /// The `run` method is used to execute the implementation
    fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain);
}

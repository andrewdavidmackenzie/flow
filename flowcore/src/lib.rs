#![warn(clippy::unwrap_used)]

//! `flowcore` defines some core structs and traits used by other flow libraries and implementations

#[cfg(feature = "code")]
#[macro_use]
#[cfg(feature = "code")]
extern crate error_chain;

use std::panic::{RefUnwindSafe, UnwindSafe};

use serde_json::Value;

#[cfg(feature = "code")]
/// `content` are the content providers for files and http/https
mod content;
#[cfg(feature = "code")]
/// `function` defines functions that form part of a flow
pub mod function;
#[cfg(feature = "code")]
/// `input` defines the struct for inputs to functions in a flow
pub mod input;
#[cfg(feature = "code")]
/// `lib_manifest` defines the structs for specifying a Library's manifest and methods to load it
pub mod lib_manifest;
#[cfg(feature = "code")]
/// `lib_provider` is used to resolve library references of the type "lib://" using lib search path
pub mod lib_provider;
#[cfg(feature = "code")]
/// `manifest` is the struct that specifies the manifest of functions in a flow
pub mod manifest;
#[cfg(feature = "code")]
/// `output_connection` defines a struct for a function's output connection
pub mod output_connection;
#[cfg(feature = "code")]
/// Utility functions related to Urls
pub mod url_helper;

/// We'll put our errors in an `errors` module, and other modules in this crate will `use errors::*;`
/// to get access to everything `error_chain!` creates.
#[cfg(feature = "code")]
#[doc(hidden)]
pub mod errors {
    // Create the Error, ErrorKind, ResultExt, and Result types
    error_chain! {}
}

/// Implementations should return a value of type `RunAgain` to indicate if it should be
/// executed more times in the future.
pub type RunAgain = bool;
/// Use `RUN_AGAIN` to indicate that a function can be executed more times
pub const RUN_AGAIN: RunAgain = true;
/// Use `DONT_RUN_AGAIN` to indicate that a function should not be executed more times
pub const DONT_RUN_AGAIN: RunAgain = false;

#[doc(hidden)]
#[cfg(feature = "code")]
error_chain! {
    types {
        Error, ErrorKind, ResultExt, Result;
    }

    foreign_links {
        Url(url::ParseError);
        Io(std::io::Error);
        Serde(serde_json::error::Error);
        Recv(std::sync::mpsc::RecvError);
    }
}

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
///     A compare operator that takes two numbers and outputs the comparisons between them
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
pub trait Implementation: RefUnwindSafe + UnwindSafe + Sync + Send {
    /// The `run` method is used to execute the implementation
    fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain);
}

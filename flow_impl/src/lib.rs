#![deny(missing_docs)]
//! `flow_impl` is a derive macro that inserts code to allow a flow "implementation"
//! to be called when compiled to wasm32
//!
use std::panic::{RefUnwindSafe, UnwindSafe};

use serde_json::Value;

pub type RunAgain = bool;
pub const RUN_AGAIN: RunAgain = true;
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
/// extern crate core;
/// extern crate flow_impl;
/// #[macro_use]
/// extern crate serde_json;
///
/// use flow_impl::{Implementation, RUN_AGAIN, RunAgain};
/// use serde_json::Value;
///
/// pub struct Compare;
///
/// /*
///     A compare operator that takes two numbers and outputs the comparisons between them
/// */
/// impl Implementation for Compare {
///     fn run(&self, mut inputs: Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
///         let left = inputs[0].remove(0).as_i64().unwrap();
///         let right = inputs[1].remove(0).as_i64().unwrap();
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
///
/// # fn main() {
/// # }
/// ```
pub trait Implementation : RefUnwindSafe + UnwindSafe + Sync + Send {
    fn run(&self, inputs: Vec<Vec<Value>>) -> (Option<Value>, RunAgain);
}
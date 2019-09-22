use std::panic::{RefUnwindSafe, UnwindSafe};

use serde_json::Value;

pub type RunAgain = bool;
pub const RUN_AGAIN: RunAgain = true;
pub const DONT_RUN_AGAIN: RunAgain = false;

/// # `flow_impl` trait
/// Define a trait that implementations of flow 'functions' must implement in order for
/// them to be invoked by the flowrlib (or other) runtime library.
///
/// ## Derive Macro
/// The flow_impl_derive crate defines a Derive macro called `FlowImpl` that should be used on
/// the structure that implements the function, in order that when compiled for the `wasm32`
/// target. code is inserted to allocate memory (`alloc`) and to serialize and deserialize the
/// data passed across the native/wasm boundary.
///
/// Example implementation of this trait:
/// ```
/// extern crate core;
/// extern crate flowrlib;
/// #[macro_use]
/// extern crate serde_json;
///
/// use flowrlib::implementation::{Implementation, RUN_AGAIN, RunAgain};
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
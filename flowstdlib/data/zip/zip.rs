extern crate core;
extern crate flow_impl;
extern crate flow_impl_derive;
#[cfg(target_arch = "wasm32")]
#[macro_use]
extern crate serde_json;

use flow_impl::{Implementation, RUN_AGAIN, RunAgain};
use flow_impl_derive::FlowImpl;
use serde_json::Value;

#[derive(FlowImpl)]
/// The struct for `Zip` implementation
pub struct Zip;

impl Implementation for Zip {
    fn run(&self, mut inputs: Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        let left = inputs.remove(0);
        let right = inputs.remove(0);

        let tuples = left.iter().zip(right.iter());

        let tuples_vec: Vec<(&Value, &Value)> = tuples.collect();
        (Some(json!(tuples_vec)), RUN_AGAIN)
    }
}
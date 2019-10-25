#[cfg(target_arch = "wasm32")]
extern crate core;
#[cfg(target_arch = "wasm32")]
extern crate flow_impl;
#[cfg(target_arch = "wasm32")]
extern crate flow_impl_derive;
#[cfg(target_arch = "wasm32")]
extern crate serde_json;

use flow_impl::{Implementation, RUN_AGAIN, RunAgain};
use flow_impl_derive::FlowImpl;
use serde_json::Value;

#[derive(FlowImpl)]
/// The struct for `Compare` implementation
pub struct Compare;

/*
    A compare operator that takes two numbers (for now) and outputs the comparisons between them
*/
impl Implementation for Compare {
    fn run(&self, mut inputs: Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        let left = inputs[0].remove(0).as_i64().unwrap();
        let right = inputs[1].remove(0).as_i64().unwrap();

        let mut output_map = serde_json::Map::new();
        output_map.insert("equal".into(), Value::Bool(left == right));
        output_map.insert("lt".into(), Value::Bool(left < right));
        output_map.insert("gt".into(), Value::Bool(left > right));
        output_map.insert("lte".into(), Value::Bool(left <= right));
        output_map.insert("gte".into(), Value::Bool(left >= right));
        let output = Value::Object(output_map);

        (Some(output), RUN_AGAIN)
    }
}
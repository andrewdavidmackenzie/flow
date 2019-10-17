extern crate core;
extern crate flow_impl;
extern crate flow_impl_derive;
#[macro_use]
extern crate serde_json;

use flow_impl::{Implementation, RUN_AGAIN, RunAgain};
use flow_impl_derive::FlowImpl;
use serde_json::Value;

#[derive(FlowImpl)]
pub struct Compare;

/*
    A compare operator that takes two numbers (for now) and outputs the comparisons between them
*/
impl Implementation for Compare {
    fn run(&self, mut inputs: Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        let left = inputs[0].remove(0).as_i64().unwrap();
        let right = inputs[1].remove(0).as_i64().unwrap();

        let output = json!({
                    "equal" : left == right,
                    "lt" : left < right,
                    "gt" : left > right,
                    "lte" : left <= right,
                    "gte" : left >= right,
                });

        (Some(output), RUN_AGAIN)
    }
}
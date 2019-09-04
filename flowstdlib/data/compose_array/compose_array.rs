extern crate core;
extern crate flow_impl;
extern crate flow_impl_derive;
#[macro_use]
extern crate serde_json;

use flow_impl::implementation::{Implementation, RUN_AGAIN, RunAgain};
use flow_impl_derive::FlowImpl;
use serde_json::Value;

#[derive(FlowImpl)]
pub struct ComposeArray;

impl Implementation for ComposeArray {
    fn run(&self, mut inputs: Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        let mut input_stream = inputs.remove(0);
        let a = input_stream.remove(0);
        let b = input_stream.remove(0);
        let c = input_stream.remove(0);
        let d = input_stream.remove(0);

        (Some(json!([a, b, c, d])), RUN_AGAIN)
    }
}
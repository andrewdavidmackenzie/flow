use serde_json::json;
use serde_json::Value;

use flow_impl_derive::FlowImpl;
use flowcore::{Implementation, RUN_AGAIN, RunAgain};

#[derive(FlowImpl)]
/// Takes two arrays of values and produce an array of tuples of pairs of values from each input array.
#[derive(Debug)]
pub struct Zip;

impl Implementation for Zip {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain) {
        let left = &inputs[0].as_array().unwrap();
        let right = &inputs[1].as_array().unwrap();

        let tuples = left.iter().zip(right.iter());

        let tuples_vec: Vec<(&Value, &Value)> = tuples.collect();
        (Some(json!(tuples_vec)), RUN_AGAIN)
    }
}

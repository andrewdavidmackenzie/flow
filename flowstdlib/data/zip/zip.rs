use serde_json::json;
use serde_json::Value;

use flow_impl_derive::FlowImpl;
use flowcore::{Implementation, RunAgain, RUN_AGAIN};

#[derive(FlowImpl)]
/// Takes two arrays of values and produce an array of tuples of pairs of values from each input array.
#[derive(Debug)]
pub struct Zip;

impl Implementation for Zip {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain) {
        if let Some(left) = &inputs[0].as_array() {
            if let Some(right) = &inputs[1].as_array() {
                let tuples = left.iter().zip(right.iter());
                let tuples_vec: Vec<(&Value, &Value)> = tuples.collect();
                return (Some(json!(tuples_vec)), RUN_AGAIN);
            }
        }

        (None, RUN_AGAIN)
    }
}

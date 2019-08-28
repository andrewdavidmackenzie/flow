use serde_json::Value;

use flow_impl::implementation::{Implementation, RUN_AGAIN, RunAgain};

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
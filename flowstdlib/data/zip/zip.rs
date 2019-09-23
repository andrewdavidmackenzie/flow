extern crate flow_impl_derive;
#[macro_use]
extern crate serde_json;

use flow_impl_derive::FlowImpl;
use serde_json::Value;

#[derive(FlowImpl)]
pub struct Zip;

impl Zip {
    fn run(&self, mut inputs: Vec<Vec<Value>>) -> (Option<Value>, bool) {
        let left = inputs.remove(0);
        let right = inputs.remove(0);

        let tuples = left.iter().zip(right.iter());

        let tuples_vec: Vec<(&Value, &Value)> = tuples.collect();
        (Some(json!(tuples_vec)), true)
    }
}
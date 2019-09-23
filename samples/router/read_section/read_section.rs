extern crate core;
extern crate flow_impl_derive;
#[macro_use]
extern crate serde_json;

use flow_impl_derive::FlowImpl;
use serde_json::Value;

#[derive(FlowImpl)]
pub struct Router;

impl Router {
    fn run(&self, mut inputs: Vec<Vec<Value>>) -> (Option<Value>, bool) {
        let mut value = None;

        let input_stream = inputs.remove(0);
        let ra = input_stream[0].as_str().unwrap().parse::<u64>();
        let rb = input_stream[1].as_str().unwrap().parse::<u64>();
        let rc = input_stream[2].as_str().unwrap().parse::<u64>();

        match (ra, rb, rc) {
            (Ok(a), Ok(b), Ok(c)) => {
                value = Some(json!(format!("{:?}", [a, b, c]))); // TODO remove format
            }
            _ => {}
        }

        (value, true)
    }
}
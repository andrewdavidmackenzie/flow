extern crate core;
extern crate flow_impl_derive;

use flow_impl_derive::FlowImpl;
use serde_json::Value;

#[derive(FlowImpl)]
pub struct Join;

/*
    A function that outputs the "data" input once the second input "control" is available and
    the function can run
*/
impl Join {
    fn run(&self, mut inputs: Vec<Vec<Value>>) -> (Option<Value>, bool) {
        let data = Some(inputs[0].remove(0));

        (data, true)
    }
}
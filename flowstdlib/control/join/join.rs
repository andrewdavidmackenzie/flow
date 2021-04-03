use serde_json::Value;

use flow_impl_derive::FlowImpl;
use flowcore::{Implementation, RunAgain, RUN_AGAIN};

#[derive(FlowImpl)]
/// Control the flow of a piece of data by waiting for a second value to be available
#[derive(Debug)]
pub struct Join;

impl Implementation for Join {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain) {
        let data = Some(inputs[0].clone());

        (data, RUN_AGAIN)
    }
}

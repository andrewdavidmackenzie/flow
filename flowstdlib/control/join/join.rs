use flow_impl::{Implementation, RUN_AGAIN, RunAgain};
use flow_impl_derive::FlowImpl;
use serde_json::Value;

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
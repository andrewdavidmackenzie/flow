use serde_json::Value;

use flow_impl_derive::FlowImpl;
use flowcore::{Implementation, RunAgain, RUN_AGAIN};

#[derive(FlowImpl)]
/// Control the flow of data (flow or disapear it) based on a boolean control value.
#[derive(Debug)]
pub struct Tap;

impl Implementation for Tap {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain) {
        let mut value = None;
        let data = &inputs[0];
        let control = &inputs[1].as_bool().unwrap();
        if *control {
            value = Some(data.clone());
        }

        (value, RUN_AGAIN)
    }
}

use flow_impl::{Implementation, RUN_AGAIN, RunAgain};
use flow_impl_derive::FlowImpl;
use serde_json::Value;

#[derive(FlowImpl)]
/// Convert a String to Json
#[derive(Debug)]
pub struct ToJson;

impl Implementation for ToJson {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain) {
        let input = &inputs[0];

        if input.is_null() {
            (Some(Value::Null), RUN_AGAIN)
        } else if input.is_string() {
            (Some(serde_json::from_str(input.as_str().unwrap()).unwrap()), RUN_AGAIN)
        } else {
            (Some(input.clone()), RUN_AGAIN)
        }
    }
}
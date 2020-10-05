use flow_impl::{Implementation, RUN_AGAIN, RunAgain};
use flow_impl_derive::FlowImpl;
use serde_json::Value;

#[derive(FlowImpl)]
/// Convert an input type to a String
#[derive(Debug)]
pub struct ToString;

impl Implementation for ToString {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain) {
        let input = &inputs[0];
        match input {
            Value::String(_) => (Some(input.clone()), RUN_AGAIN),
            Value::Bool(boolean) => (Some(Value::String(boolean.to_string())), RUN_AGAIN),
            Value::Number(number) => (Some(Value::String(number.to_string())), RUN_AGAIN),
            _ => (None, RUN_AGAIN)
        }
    }
}
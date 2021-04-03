use serde_json::json;
use serde_json::Value;
use serde_json::Value::String as JsonString;

use flow_impl_derive::FlowImpl;
use flowcore::{Implementation, RunAgain, RUN_AGAIN};

#[derive(FlowImpl)]
/// Reverse a String
#[derive(Debug)]
pub struct Reverse;

impl Implementation for Reverse {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain) {
        let mut value = None;

        let input = &inputs[0];
        if let JsonString(ref s) = input {
            value = Some(json!({
                "reversed" : s.chars().rev().collect::<String>(),
                "original": s
            }));
        }

        (value, RUN_AGAIN)
    }
}

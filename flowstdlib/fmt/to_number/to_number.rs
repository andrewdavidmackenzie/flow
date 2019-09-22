extern crate core;
extern crate flow_impl_derive;
extern crate flowrlib;
extern crate serde_json;

use flow_impl_derive::FlowImpl;
use flowrlib::implementation::{Implementation, RUN_AGAIN, RunAgain};
use serde_json::Value;

#[derive(FlowImpl)]
pub struct ToNumber;

impl Implementation for ToNumber {
    fn run(&self, mut inputs: Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        let mut value = None;
        let input = inputs.remove(0).remove(0);

        match input {
            Value::String(string) => {
                if let Ok(number) = string.parse::<i64>() {
                    let number = Value::Number(serde_json::Number::from(number));
                    value = Some(number);
                }
            },
            _ => {}
        };

        (value, RUN_AGAIN)
    }
}
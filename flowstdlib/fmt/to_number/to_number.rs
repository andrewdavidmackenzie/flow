extern crate flow_impl_derive;
extern crate serde_json;

use flow_impl_derive::FlowImpl;
use serde_json::Value;

#[derive(FlowImpl)]
pub struct ToNumber;

impl ToNumber {
    fn run(&self, mut inputs: Vec<Vec<Value>>) -> (Option<Value>, bool) {
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

        (value, true)
    }
}
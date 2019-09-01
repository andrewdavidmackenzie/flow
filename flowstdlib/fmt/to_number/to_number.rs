use flow_impl::implementation::{Implementation, RunAgain};
use serde_json;
use serde_json::Value;

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

        (value, true)
    }
}
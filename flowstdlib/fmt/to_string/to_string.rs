use flow_impl::implementation::{Implementation, RunAgain};
use serde_json::Value;

pub struct ToString;

impl Implementation for ToString {
    fn run(&self, mut inputs: Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        let mut value = None;

        let input = inputs.remove(0).remove(0);
        match input {
            Value::String(_) => {
                value = Some(input);
            },
            Value::Bool(boolean) => {
                let val = Value::String(boolean.to_string());
                value = Some(val);
            },
            Value::Number(number) => {
                let val = Value::String(number.to_string());
                value = Some(val);
            },
            _ => {}
        };

        (value, true)
    }
}
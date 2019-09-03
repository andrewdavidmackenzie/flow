use serde_json::Value;

use flow_impl::implementation::{Implementation, RUN_AGAIN, RunAgain};

#[derive(Debug)]
pub struct Stderr;

impl Implementation for Stderr {
    fn run(&self, mut inputs: Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        let input = inputs.remove(0).remove(0);
        match input {
            Value::String(string) => {
                eprintln!("{}", string);
            },
            Value::Bool(boolean) => {
                eprintln!("{}", Value::String(boolean.to_string()));
            },
            Value::Number(number) => {
                eprintln!("{}", Value::String(number.to_string()));
            },
            Value::Array(array) => {
                for entry in array {
                    eprintln!("{}", entry);
                }
            },
            _ => {}
        };

        (None, RUN_AGAIN)
    }
}
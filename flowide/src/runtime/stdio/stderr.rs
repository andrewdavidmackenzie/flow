use flow_impl::{Implementation, RUN_AGAIN, RunAgain};
use serde_json::Value;

pub struct Stderr;

impl Implementation for Stderr {
    fn run(&self, mut inputs: Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        let input = inputs.remove(0).remove(0);

        // TODO

        match input {
            Value::String(string) => {
                eprintln!("{}", string);
            },
            Value::Bool(boolean) => {
                eprintln!("{}", boolean);
            },
            Value::Number(number) => {
                eprintln!("{}", number);
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
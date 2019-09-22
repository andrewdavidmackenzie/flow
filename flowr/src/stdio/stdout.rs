use serde_json::Value;

use flowrlib::implementation::{Implementation, RUN_AGAIN, RunAgain};

#[derive(Debug)]
pub struct Stdout;

impl Implementation for Stdout {
    fn run(&self, mut inputs: Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        let input = inputs.remove(0).remove(0);
        match input {
            Value::String(string) => {
                println!("{}", string);
            },
            Value::Bool(boolean) => {
                println!("{}", Value::String(boolean.to_string()));
            },
            Value::Number(number) => {
                println!("{}", Value::String(number.to_string()));
            },
            Value::Array(array) => {
                for entry in array {
                    println!("{}", entry);
                }
            },
            _ => {}
        };

    (None, RUN_AGAIN)
    }
}
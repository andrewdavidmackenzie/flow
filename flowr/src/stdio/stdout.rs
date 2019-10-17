use flow_impl::{Implementation, RUN_AGAIN, RunAgain};
use serde_json::Value;

#[derive(Debug)]
/// `Implementation` struct for the `stdout` function
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
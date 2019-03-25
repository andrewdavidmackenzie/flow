use flowrlib::implementation::Implementation;
use flowrlib::implementation::RUN_AGAIN;
use flowrlib::implementation::RunAgain;
use serde_json::Value;

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
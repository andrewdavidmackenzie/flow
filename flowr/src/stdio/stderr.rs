use flowrlib::implementation::Implementation;
use flowrlib::implementation::RUN_AGAIN;
use flowrlib::implementation::RunAgain;
use serde_json::Value as JsonValue;

pub struct Stderr;

impl Implementation for Stderr {
    fn run(&self, mut inputs: Vec<Vec<JsonValue>>) -> (Option<JsonValue>, RunAgain) {
        let input = inputs.remove(0).remove(0);
        match input {
            JsonValue::String(string) => {
                eprintln!("{}", string);
            },
            JsonValue::Bool(boolean) => {
                eprintln!("{}", JsonValue::String(boolean.to_string()));
            },
            JsonValue::Number(number) => {
                eprintln!("{}", JsonValue::String(number.to_string()));
            },
            JsonValue::Array(array) => {
                for entry in array {
                    eprintln!("{}", entry);
                }
            },
            _ => {}
        };

        (None, RUN_AGAIN)
    }
}
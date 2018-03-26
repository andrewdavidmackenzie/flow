use serde_json::Value as JsonValue;
use flowrlib::implementation::Implementation;

pub struct Readline;

impl Implementation for Readline {
    fn run(&self, _inputs: Vec<JsonValue>) -> JsonValue {
        use std::io::{self, BufRead};

        let stdin = io::stdin();
        let mut iterator = stdin.lock().lines();
        if let Some(result) = iterator.next() {
            if let Ok(line) = result {
                return JsonValue::String(line);
            }
        }
        JsonValue::Null
    }
}
use serde_json::Value as JsonValue;
use flowrlib::implementation::Implementation;

pub struct Stdin;

impl Implementation for Stdin {
    fn run(&self, _inputs: Vec<JsonValue>) -> JsonValue {
        use std::io::{self, Read};

        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer).unwrap();
        JsonValue::String(buffer)
    }
}
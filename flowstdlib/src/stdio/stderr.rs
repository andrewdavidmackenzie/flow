use serde_json::Value as JsonValue;
use flowrlib::implementation::Implementation;

pub struct Stderr;

impl Implementation for Stderr {
    fn run(&self, mut inputs: Vec<JsonValue>) -> JsonValue {
        eprintln!("{}", inputs.remove(0).as_str().unwrap());
        JsonValue::Null
    }
}
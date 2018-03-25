use serde_json::Value as JsonValue;
use flowrlib::implementation::Implementation;

pub struct Stdout;

impl Implementation for Stdout {
    fn run(&self, mut inputs: Vec<JsonValue>) -> JsonValue {
        println!("{}", inputs.remove(0).as_str().unwrap());
        JsonValue::Null
    }
}
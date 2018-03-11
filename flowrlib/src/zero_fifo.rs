use serde_json::Value as JsonValue;
use implementation::Implementation;

pub struct Fifo;

impl Implementation for Fifo {
    fn run(&self, mut inputs: Vec<JsonValue>) -> JsonValue {
        inputs.remove(0)
    }
}
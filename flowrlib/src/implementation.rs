use serde_json::Value as JsonValue;

pub trait Implementation {
    // An implementation runs, receiving an array of inputs and possibly producing an output
    fn run(&self, inputs: Vec<JsonValue>) -> JsonValue;
}
use serde_json::Value as JsonValue;
use std::panic::RefUnwindSafe;
use std::panic::UnwindSafe;

pub trait Implementation : RefUnwindSafe + UnwindSafe {
    // An implementation runs, receiving an array of inputs and possibly producing an output
    fn run(&self, inputs: Vec<JsonValue>) -> JsonValue;
}
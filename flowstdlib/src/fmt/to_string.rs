use serde_json::Value as JsonValue;
use serde_json::Value::Number;
use flowrlib::implementation::Implementation;

pub struct ToString;

impl Implementation for ToString {
    fn run(&self, mut inputs: Vec<JsonValue>) -> JsonValue {
        let input = inputs.remove(0);
        match &input {
            &Number(ref n) => JsonValue::String(n.to_string()),
            _ => JsonValue::Null
        }
    }
}
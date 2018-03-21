use serde_json::Value as JsonValue;
use serde_json::Value::String as JsonString;
use flowrlib::implementation::Implementation;

pub struct Reverse;

impl Implementation for Reverse {
    fn run(&self, mut inputs: Vec<JsonValue>) -> JsonValue {
        let input = inputs.remove(0);
        match &input {
            &JsonString(ref s) => json!({
                "reversed" : s.chars().rev().collect::<String>(),
                "original": s
            }),
            _ => JsonValue::Null
        }
    }
}
use flowrlib::implementation::Implementation;
use flowrlib::implementation::RunAgain;
use serde_json;
use serde_json::Value as JsonValue;

pub struct ToNumber;

impl Implementation for ToNumber {
    fn run(&self, mut inputs: Vec<Vec<JsonValue>>) -> (Option<JsonValue>, RunAgain) {
        let mut value = None;
        let input = inputs.remove(0).remove(0);

        match input {
            JsonValue::String(string) => {
                if let Ok(number) = string.parse::<i64>() {
                    let number = JsonValue::Number(serde_json::Number::from(number));
                    value = Some(number);
                }
            },
            _ => {}
        };

        (value, true)
    }
}
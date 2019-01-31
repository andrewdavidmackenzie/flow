#[macro_use]
extern crate serde_json;

use serde_json::Value as JsonValue;
use serde_json::Value::String as JsonString;

#[no_mangle]
pub extern "C" fn reverse(mut inputs: Vec<Vec<JsonValue>>) -> (Option<JsonValue>, bool) {
    let mut value = None;

    let input = inputs.remove(0).remove(0);
    match input {
        JsonString(ref s) => {
            value = Some(json!({
                    "reversed" : s.chars().rev().collect::<String>(),
                    "original": s
                }));
        }
        _ => {}
    }

    (value, true)
}
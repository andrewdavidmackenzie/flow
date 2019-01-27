extern crate flowrlib;
#[macro_use] extern crate serde_json;

use flowrlib::implementation::Implementation;
use flowrlib::implementation::RUN_AGAIN;
use flowrlib::implementation::RunAgain;
use serde_json::Value as JsonValue;
use serde_json::Value::String as JsonString;

pub struct Reverse;

impl Implementation for Reverse {
    fn run(&self, mut inputs: Vec<Vec<JsonValue>>) -> (Option<JsonValue>, RunAgain) {
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

        (value, RUN_AGAIN)
    }
}
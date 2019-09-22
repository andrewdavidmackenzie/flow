extern crate core;
extern crate flow_impl_derive;
extern crate flowrlib;
#[macro_use]
extern crate serde_json;

use flow_impl_derive::FlowImpl;
use flowrlib::implementation::{Implementation, RUN_AGAIN, RunAgain};
use serde_json::Value;
use serde_json::Value::String as JsonString;

#[derive(FlowImpl)]
pub struct Reverse;

impl Implementation for Reverse {
    fn run(&self, mut inputs: Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
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
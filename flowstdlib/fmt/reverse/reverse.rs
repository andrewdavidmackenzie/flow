extern crate core;
extern crate flow_impl;
extern crate flow_impl_derive;
#[cfg(target_arch = "wasm32")]
#[macro_use]
extern crate serde_json;

use flow_impl::{Implementation, RUN_AGAIN, RunAgain};
use flow_impl_derive::FlowImpl;
use serde_json::Value;
use serde_json::Value::String as JsonString;

#[derive(FlowImpl)]
/// The struct for `Reverse` implementation
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
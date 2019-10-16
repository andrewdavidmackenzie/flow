extern crate core;
extern crate flow_impl;
extern crate flow_impl_derive;
#[macro_use]
extern crate serde_json;

use flow_impl::Implementation;
use flow_impl_derive::FlowImpl;
use serde_json::Value;
use serde_json::Value::String as JsonString;

#[derive(FlowImpl)]
pub struct Reverser;

impl Implementation for Reverser {
    fn run(&self, mut inputs: Vec<Vec<Value>>) -> (Option<Value>, bool) {
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
}
extern crate core;
extern crate flow_impl;
extern crate flow_impl_derive;
#[macro_use]
extern crate serde_json;

use flow_impl::implementation::{Implementation, RUN_AGAIN, RunAgain};
use flow_impl_derive::FlowImpl;
use serde_json::Value;
use serde_json::Value::Number;
use serde_json::Value::String;

#[derive(FlowImpl)]
pub struct Add;

// TODO implementation of `std::ops::Add` might be missing for `&serde_json::Number`

impl Implementation for Add {
    fn run(&self, inputs: Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        let mut value = None;

        let input_a = inputs.get(0).unwrap();
        let input_b = inputs.get(1).unwrap();
        match (&input_a[0], &input_b[0]) {
            (&Number(ref a), &Number(ref b)) => {
                value = Some(Value::Number(serde_json::Number::from(a.as_i64().unwrap().checked_add(b.as_i64().unwrap()).unwrap())));
            }
            (&String(ref a), &String(ref b)) => {
                let i1 = a.parse::<i64>().unwrap();
                let i2 = b.parse::<i64>().unwrap();
                let o1 = i1 + i2;
                value = Some(Value::String(o1.to_string()));
            }
            (_, _) => {}
        }

        (value, RUN_AGAIN)
    }
}
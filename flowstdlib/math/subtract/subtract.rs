extern crate core;
extern crate flow_impl_derive;
extern crate serde_json;

use flow_impl_derive::FlowImpl;
use serde_json::Value;
use serde_json::Value::Number;

#[derive(FlowImpl)]
pub struct Subtract;

impl Subtract {
    fn run(&self, inputs: Vec<Vec<Value>>) -> (Option<Value>, bool) {
        let input_a = inputs.get(0).unwrap();
        let input_b = inputs.get(1).unwrap();
        let mut value = None;

        match (&input_a[0], &input_b[0]) {
            (&Number(ref a), &Number(ref b)) => {
                // TODO mixed signed and unsigned integers
                if a.is_i64() && b.is_i64() {
                    value = Some(Value::Number(serde_json::Number::from(a.as_i64().unwrap() - b.as_i64().unwrap())));
                } else if a.is_u64() && b.is_u64() {
                    value = Some(Value::Number(serde_json::Number::from(a.as_u64().unwrap() - b.as_u64().unwrap())));
                } else if a.is_f64() && b.is_f64() {
                    value = Some(Value::Number(serde_json::Number::from_f64(a.as_f64().unwrap() - b.as_f64().unwrap()).unwrap()));
                }
            }
            (_, _) => {}
        }

        (value, true)
    }
}
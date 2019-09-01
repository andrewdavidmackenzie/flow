use flow_impl::implementation::{Implementation, RunAgain};
use serde_json;
use serde_json::Value;
use serde_json::Value::Number;

pub struct Subtract;

// TODO implementation of `std::ops::Add` might be missing for `&serde_json::Number`

impl Implementation for Subtract {
    fn run(&self, inputs: Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
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
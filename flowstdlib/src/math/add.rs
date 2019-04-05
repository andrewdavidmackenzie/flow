use flowrlib::implementation::Implementation;
use flowrlib::implementation::RunAgain;
use serde_json;
use serde_json::Value;
use serde_json::Value::Number;
use serde_json::Value::String;

pub struct Add;

// TODO implementation of `std::ops::Add` might be missing for `&serde_json::Number`

impl Implementation for Add {
    fn run(&self, inputs: Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        let mut value = None;

        let input_a = inputs.get(0).unwrap();
        let input_b = inputs.get(1).unwrap();
        match (&input_a[0], &input_b[0]) {
            (&Number(ref a), &Number(ref b)) => {
                // TODO mixed signed and unsigned integers
                if a.is_i64() && b.is_i64() {
                    value = Some(Value::Number(serde_json::Number::from(a.as_i64().unwrap() + b.as_i64().unwrap())));
                } else if a.is_u64() && b.is_u64() {
                    value = Some(Value::Number(serde_json::Number::from(a.as_u64().unwrap() + b.as_u64().unwrap())));
                } else if a.is_f64() || b.is_f64() {
                    value = Some(Value::Number(serde_json::Number::from_f64(a.as_f64().unwrap() + b.as_f64().unwrap()).unwrap()));
                }
            }
            (&String(ref a), &String(ref b)) => {
                let i1 = a.parse::<i32>().unwrap();
                let i2 = b.parse::<i32>().unwrap();
                let o1 = i1 + i2;
                value = Some(Value::String(o1.to_string()));
            }
            (_, _) => {}
        }

        (value, true)
    }
}
use serde_json;
use serde_json::Value as JsonValue;
use serde_json::Value::Number;
use serde_json::Value::String;
use flowrlib::implementation::Implementation;

pub struct Add;

// TODO implementation of `std::ops::Add` might be missing for `&serde_json::Number`

impl Implementation for Add {
    fn run(&self, inputs: Vec<JsonValue>) -> JsonValue {
        match (&inputs[0], &inputs[1]) {
            (&Number(ref a), &Number(ref b)) => {
                // TODO mixed signed and unsigned integers
                if a.is_i64() && b.is_i64() {
                    JsonValue::Number(serde_json::Number::from(a.as_i64().unwrap() + b.as_i64().unwrap()))
                } else if a.is_u64() && b.is_u64() {
                    JsonValue::Number(serde_json::Number::from(a.as_u64().unwrap() + b.as_u64().unwrap()))
                } else if a.is_f64() && b.is_f64() {
                    JsonValue::Number(serde_json::Number::from_f64(a.as_f64().unwrap() + b.as_f64().unwrap()).unwrap())
                } else {
                    JsonValue::Null
                }
            },
            (&String(ref a), &String(ref b)) => {
                let i1 = a.parse::<i32>().unwrap();
                let i2 = b.parse::<i32>().unwrap();
                let o1 = i1 + i2;
                JsonValue::String(o1.to_string())
            }
                (_, _) => JsonValue::Null
        }
    }
}
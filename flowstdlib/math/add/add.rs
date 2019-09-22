extern crate core;
extern crate flow_impl_derive;
extern crate flowrlib;
#[cfg(test)]
#[macro_use]
extern crate serde_json;
#[cfg(not(test))]
extern crate serde_json;

use flow_impl_derive::FlowImpl;
use flowrlib::implementation::{Implementation, RUN_AGAIN, RunAgain};
use serde_json::Value;
use serde_json::Value::Number;
use serde_json::Value::String;

#[derive(FlowImpl)]
pub struct Add;

impl Implementation for Add {
    fn run(&self, inputs: Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        let mut value = None;

        let input_a = inputs.get(0).unwrap();
        let input_b = inputs.get(1).unwrap();
        match (&input_a[0], &input_b[0]) {
            (&Number(ref a), &Number(ref b)) => {
                value = if a.is_i64() && b.is_i64() {
                    match a.as_i64().unwrap().checked_add(b.as_i64().unwrap()) {
                        Some(result) => Some(Value::Number(serde_json::Number::from(result))),
                        None => None
                    }
                } else if a.is_u64() && b.is_u64() {
                    match a.as_u64().unwrap().checked_add(b.as_u64().unwrap()) {
                        Some(result) => Some(Value::Number(serde_json::Number::from(result))),
                        None => None
                    }
                } else if a.is_f64() || b.is_f64() {
                    Some(Value::Number(serde_json::Number::from_f64(a.as_f64().unwrap() + b.as_f64().unwrap()).unwrap()))
                } else {
                    None
                };
            }
            (&String(ref a), &String(ref b)) => {
                let i1 = a.parse::<i64>().unwrap();
                let i2 = b.parse::<i64>().unwrap();
                let o1 = i1 + i2;
                value = Some(Value::String(o1.to_string()));
            }
            (_, _) => println!("Unsupported input value types")
        }

        (value, RUN_AGAIN)
    }
}

#[cfg(test)]
mod test {
    use flowrlib::implementation::Implementation;
    use serde_json::Value;

    use super::Add;

    fn get_inputs(pair: &(Value, Value, Option<Value>)) -> Vec<Vec<Value>> {
        vec!(vec!(pair.0.clone()), vec!(pair.1.clone()))
    }

    // 0 plus 0
    // 0 plus negative
    // negative plus 0
    // 0 plus positive
    // positive plus zero
    // positive plus positive
    // negative plus negative
    // positive plus negative
    // negative plus positive
    // overflow positive
    // overflow negative

    // all of those for:
    // integer + integer
    // float plus float
    // float plus integer
    // integer plus float

    // all of those as numbers and strings
    #[test]
    fn test_adder() {
        let integer_test_set = vec!(
            (json!(0), json!(0), Some(json!(0))),
            (json!(0), json!(-10), Some(json!(-10))),
            (json!(-10), json!(0), Some(json!(-10))),
            (json!(0), json!(10), Some(json!(10))),
            (json!(10), json!(0), Some(json!(10))),
            (json!(10), json!(20), Some(json!(30))),
            (json!(-10), json!(-20), Some(json!(-30))),
            (json!(10), json!(-20), Some(json!(-10))),
            (json!(-10), json!(20), Some(json!(10))),
            (json!(4660046610375530309 as i64), json!(7540113804746346429 as i64), None),
            (json!(-4660046610375530309 as i64), json!(-7540113804746346429 as i64), None),
        );

        let added = Add {};

        for ref test in integer_test_set {
            println!("Testing add of {:?}", test);
            let (value, again) = added.run(get_inputs(test));

            assert_eq!(true, again);
            assert_eq!(test.2, value);
        }
    }
}
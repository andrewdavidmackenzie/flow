use serde_json::Value;
use serde_json::Value::Number;

use flow_impl_derive::FlowImpl;
use flowcore::{Implementation, RunAgain, RUN_AGAIN};

#[derive(FlowImpl)]
/// Add two inputs to produce a new output
#[derive(Debug)]
pub struct Add;

impl Implementation for Add {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain) {
        let mut sum = None;

        let input_a = &inputs[0];
        let input_b = &inputs[1];

        let mut output_map = serde_json::Map::new();

        match (&input_a, &input_b) {
            (&Number(ref a), &Number(ref b)) => {
                sum = if a.is_i64() && b.is_i64() {
                    match a.as_i64().unwrap().checked_add(b.as_i64().unwrap()) {
                        Some(result) => Some(Value::Number(serde_json::Number::from(result))),
                        None => None,
                    }
                } else if a.is_u64() && b.is_u64() {
                    match a.as_u64().unwrap().checked_add(b.as_u64().unwrap()) {
                        Some(result) => Some(Value::Number(serde_json::Number::from(result))),
                        None => None,
                    }
                } else if a.is_f64() || b.is_f64() {
                    Some(Value::Number(
                        serde_json::Number::from_f64(a.as_f64().unwrap() + b.as_f64().unwrap())
                            .unwrap(),
                    ))
                } else {
                    println!("Unsupported input types combination in 'add': {:?}", inputs);
                    None
                };
            }
            (_, _) => println!("Unsupported input types in 'add': {:?}", inputs),
        }

        if let Some(total) = sum {
            output_map.insert("sum".into(), total);
            output_map.insert("i1".into(), input_a.clone());
            output_map.insert("i2".into(), input_b.clone());

            let output = Value::Object(output_map);

            (Some(output), RUN_AGAIN)
        } else {
            (None, RUN_AGAIN)
        }
    }
}

#[cfg(test)]
mod test {
    use serde_json::Value;
    use serde_json::Value::Number;

    use flowcore::Implementation;

    use super::Add;

    fn get_inputs(pair: &(Value, Value, Option<Value>)) -> Vec<Value> {
        vec![pair.0.clone(), pair.1.clone()]
    }

    // Repeat for:
    // integer + integer
    // float plus float
    // float plus integer
    // integer plus float
    #[test]
    fn test_adder() {
        let integer_test_set = vec![
            (
                // 0 plus 0
                Number(serde_json::Number::from(0)),
                Number(serde_json::Number::from(0)),
                Some(Number(serde_json::Number::from(0))),
            ),
            (
                // 0 plus negative
                Number(serde_json::Number::from(0)),
                Number(serde_json::Number::from(-10)),
                Some(Number(serde_json::Number::from(-10))),
            ),
            (
                // negative plus 0
                Number(serde_json::Number::from(-10)),
                Number(serde_json::Number::from(0)),
                Some(Number(serde_json::Number::from(-10))),
            ),
            (
                // 0 plus positive
                Number(serde_json::Number::from(0)),
                Number(serde_json::Number::from(10)),
                Some(Number(serde_json::Number::from(10))),
            ),
            (
                // positive plus zero
                Number(serde_json::Number::from(10)),
                Number(serde_json::Number::from(0)),
                Some(Number(serde_json::Number::from(10))),
            ),
            (
                // positive plus positive
                Number(serde_json::Number::from(10)),
                Number(serde_json::Number::from(20)),
                Some(Number(serde_json::Number::from(30))),
            ),
            (
                // negative plus negative
                Number(serde_json::Number::from(-10)),
                Number(serde_json::Number::from(-20)),
                Some(Number(serde_json::Number::from(-30))),
            ),
            (
                // positive plus negative
                Number(serde_json::Number::from(10)),
                Number(serde_json::Number::from(-20)),
                Some(Number(serde_json::Number::from(-10))),
            ),
            (
                // negative plus positive
                Number(serde_json::Number::from(-10)),
                Number(serde_json::Number::from(20)),
                Some(Number(serde_json::Number::from(10))),
            ),
            (
                // overflow positive
                Number(serde_json::Number::from(4_660_046_610_375_530_309 as i64)),
                Number(serde_json::Number::from(7_540_113_804_746_346_429 as i64)),
                None,
            ),
            (
                // overflow negative
                Number(serde_json::Number::from(-4_660_046_610_375_530_309 as i64)),
                Number(serde_json::Number::from(-7_540_113_804_746_346_429 as i64)),
                None,
            ),
        ];

        let added = Add {};

        for test in &integer_test_set {
            let (output, again) = added.run(&get_inputs(test));

            assert_eq!(true, again);

            match output {
                Some(outputs) => {
                    assert_eq!(outputs.pointer("/i1").unwrap(), &test.0);
                    assert_eq!(outputs.pointer("/i2").unwrap(), &test.1);
                    assert_eq!(outputs.pointer("/sum"), test.2.as_ref());
                }
                None => {
                    assert!(test.2.is_none())
                }
            }
        }
    }
}

use serde_json::Value::Number;
use serde_json::{json, Value};

use flow_impl_derive::FlowImpl;
use flowcore::{Implementation, RunAgain, RUN_AGAIN};

#[derive(FlowImpl)]
/// Add two inputs to produce a new output
#[derive(Debug)]
pub struct Add;

impl Implementation for Add {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain) {
        let input_a = &inputs[0];
        let input_b = &inputs[1];

        let sum = match (&input_a, &input_b) {
            (&Number(ref a), &Number(ref b)) => {
                if let Some(a_i64) = a.as_i64() {
                    if let Some(b_i64) = b.as_i64() {
                        match a_i64.checked_add(b_i64) {
                            Some(result) => Some(json!(result)),
                            None => None,
                        }
                    } else {
                        None
                    }
                } else if let Some(a_u64) = a.as_u64() {
                    if let Some(b_u64) = b.as_u64() {
                        match a_u64.checked_add(b_u64) {
                            Some(result) => Some(json!(result)),
                            None => None,
                        }
                    } else {
                        None
                    }
                } else if let Some(a_f64) = a.as_f64() {
                    if let Some(b_f64) = b.as_f64() {
                        Some(json!(a_f64 + b_f64))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            (_, _) => None,
        };

        if let Some(total) = sum {
            let mut output_map = serde_json::Map::new();
            output_map.insert("sum".into(), total);
            output_map.insert("i1".into(), input_a.clone());
            output_map.insert("i2".into(), input_b.clone());
            (Some(Value::Object(output_map)), RUN_AGAIN)
        } else {
            (None, RUN_AGAIN)
        }
    }
}

#[cfg(test)]
mod test {
    use serde_json::json;
    use serde_json::Value;
    use serde_json::Value::Number;

    use flowcore::Implementation;

    use super::Add;

    fn get_inputs(pair: &(Value, Value, Option<Value>)) -> Vec<Value> {
        vec![pair.0.clone(), pair.1.clone()]
    }

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
                Number(serde_json::Number::from(4_660_046_610_375_530_309_i64)),
                Number(serde_json::Number::from(7_540_113_804_746_346_429_i64)),
                None,
            ),
            (
                // overflow negative
                Number(serde_json::Number::from(-4_660_046_610_375_530_309_i64)),
                Number(serde_json::Number::from(-7_540_113_804_746_346_429_i64)),
                None,
            ),
            (
                // force u64
                Number(serde_json::Number::from(i64::MAX as u64 + 10)),
                Number(serde_json::Number::from(i64::MAX as u64 + 10)),
                None,
            ),
            (
                // force u64 and i64
                Number(serde_json::Number::from(i64::MAX as u64 + 10)),
                Number(serde_json::Number::from(-1_i64)),
                None,
            ),
            (
                // float
                json!(1.0),
                json!(1.0),
                Some(json!(2.0)),
            ),
            (
                // invalid
                json!(1.0),
                json!("aaa"),
                None,
            ),
        ];

        let added = Add {};

        for test in &integer_test_set {
            let (output, again) = added.run(&get_inputs(test));

            assert_eq!(true, again);

            match output {
                Some(outputs) => {
                    assert_eq!(
                        outputs.pointer("/i1").expect("Could not get i1 output"),
                        &test.0
                    );
                    assert_eq!(
                        outputs.pointer("/i2").expect("Could not get i2 output"),
                        &test.1
                    );
                    assert_eq!(outputs.pointer("/sum"), test.2.as_ref());
                }
                None => {
                    assert!(test.2.is_none())
                }
            }
        }
    }
}

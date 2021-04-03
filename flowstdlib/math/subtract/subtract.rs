use serde_json::Value;
use serde_json::Value::Number;

use flow_impl_derive::FlowImpl;
use flowcore::{Implementation, RunAgain, RUN_AGAIN};

#[derive(FlowImpl)]
/// Subtract one input from another to produce a new output
#[derive(Debug)]
pub struct Subtract;

impl Implementation for Subtract {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain) {
        let input_a = &inputs[0];
        let input_b = &inputs[1];
        let mut value: Option<Value> = None;

        let mut output_map = serde_json::Map::new();

        match (&input_a, &input_b) {
            (&Number(ref a), &Number(ref b)) => {
                // TODO mixed signed and unsigned integers
                if a.is_i64() && b.is_i64() {
                    // both signed integers
                    let result = a.as_i64().unwrap().checked_sub(b.as_i64().unwrap());
                    if let Some(int) = result {
                        value = Some(Value::Number(serde_json::Number::from(int)));
                    }
                } else if a.is_u64() && b.is_u64() {
                    // both unsigned integers
                    let result = a.as_u64().unwrap().checked_sub(b.as_u64().unwrap());
                    if let Some(int) = result {
                        value = Some(Value::Number(serde_json::Number::from(int)));
                    }
                } else if a.is_f64() && b.is_f64() {
                    // both float
                    let result = a.as_f64().unwrap() - b.as_f64().unwrap();
                    if let Some(f) = serde_json::Number::from_f64(result) {
                        value = Some(Value::Number(f))
                    }
                } else {
                    println!(
                        "Unsupported input type combination in 'subtract': {:?}",
                        inputs
                    );
                };
            }
            (_, _) => {}
        }

        if let Some(diff) = value {
            output_map.insert("diff".into(), diff);
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

    use super::Subtract;

    fn get_inputs(pair: &(Value, Value, Option<Value>)) -> Vec<Value> {
        vec![pair.0.clone(), pair.1.clone()]
    }

    // repeat for:
    // integer + integer
    // float plus float
    // float plus integer
    // integer plus float
    #[test]
    fn test_subtract() {
        let integer_test_set = vec![
            (
                // 0 minus 0
                Number(serde_json::Number::from(0)),
                Number(serde_json::Number::from(0)),
                Some(Number(serde_json::Number::from(0))),
            ),
            (
                // 0 minus negative
                Number(serde_json::Number::from(0)),
                Number(serde_json::Number::from(-10)),
                Some(Number(serde_json::Number::from(10))),
            ),
            (
                // negative minus 0
                Number(serde_json::Number::from(-10)),
                Number(serde_json::Number::from(0)),
                Some(Number(serde_json::Number::from(-10))),
            ),
            (
                // 0 minus positive
                Number(serde_json::Number::from(0)),
                Number(serde_json::Number::from(10)),
                Some(Number(serde_json::Number::from(-10))),
            ),
            (
                // positive minus zero
                Number(serde_json::Number::from(10)),
                Number(serde_json::Number::from(0)),
                Some(Number(serde_json::Number::from(10))),
            ),
            (
                // positive minus positive
                Number(serde_json::Number::from(10)),
                Number(serde_json::Number::from(20)),
                Some(Number(serde_json::Number::from(-10))),
            ),
            (
                // negative minus negative
                Number(serde_json::Number::from(-10)),
                Number(serde_json::Number::from(-20)),
                Some(Number(serde_json::Number::from(10))),
            ),
            (
                // positive minus negative
                Number(serde_json::Number::from(10)),
                Number(serde_json::Number::from(-20)),
                Some(Number(serde_json::Number::from(30))),
            ),
            (
                // negative minus positive
                Number(serde_json::Number::from(-10)),
                Number(serde_json::Number::from(20)),
                Some(Number(serde_json::Number::from(-30))),
            ),
            (
                // overflow minus
                Number(serde_json::Number::from(-4_660_046_610_375_530_309 as i64)),
                Number(serde_json::Number::from(7_540_113_804_746_346_429 as i64)),
                None,
            ),
            (
                // overflow positive
                Number(serde_json::Number::from(4_660_046_610_375_530_309 as i64)),
                Number(serde_json::Number::from(-7_540_113_804_746_346_429 as i64)),
                None,
            ),
        ];

        let subtract = Subtract {};

        for test in &integer_test_set {
            let (output, again) = subtract.run(&get_inputs(test));

            assert_eq!(true, again);

            match output {
                Some(outputs) => {
                    assert_eq!(outputs.pointer("/i1").unwrap(), &test.0);
                    assert_eq!(outputs.pointer("/i2").unwrap(), &test.1);
                    assert_eq!(outputs.pointer("/diff"), test.2.as_ref());
                }
                None => {
                    assert!(test.2.is_none())
                }
            }
        }
    }
}

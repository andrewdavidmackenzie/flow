use serde_json::Value::Number;
use serde_json::{json, Value};

use flow_macro::flow_function;

#[flow_function]
fn _subtract(inputs: &[Value]) -> (Option<Value>, RunAgain) {
    let input_a = &inputs[0];
    let input_b = &inputs[1];
    let mut value: Option<Value> = None;

    match (&input_a, &input_b) {
        (&Number(ref a), &Number(ref b)) => {
            if let Some(a_i64) = a.as_i64() {
                if let Some(b_i64) = b.as_i64() {
                    let result = a_i64.checked_sub(b_i64);
                    if let Some(int) = result {
                        value = Some(json!(int));
                    }
                }
            } else if let Some(a_u64) = a.as_u64() {
                if let Some(b_u64) = b.as_u64() {
                    let result = a_u64.checked_sub(b_u64);
                    if let Some(int) = result {
                        value = Some(json!(int));
                    }
                }
            } else if let Some(a_f64) = a.as_f64() {
                if let Some(b_f64) = b.as_f64() {
                    let result = a_f64 - b_f64;
                    if let Some(f) = serde_json::Number::from_f64(result) {
                        value = Some(Value::Number(f))
                    }
                }
            };
        }
        (_, _) => {}
    }

    if let Some(diff) = value {
        (Some(json!(diff)), RUN_AGAIN)
    } else {
        (None, RUN_AGAIN)
    }
}

#[cfg(test)]
mod test {
    use serde_json::json;
    use serde_json::Value;
    use serde_json::Value::Number;

    use super::_subtract;

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
                Number(serde_json::Number::from(-4_660_046_610_375_530_309_i64)),
                Number(serde_json::Number::from(7_540_113_804_746_346_429_i64)),
                None,
            ),
            (
                // overflow positive
                Number(serde_json::Number::from(4_660_046_610_375_530_309_i64)),
                Number(serde_json::Number::from(-7_540_113_804_746_346_429_i64)),
                None,
            ),
            (
                // force u64
                Number(serde_json::Number::from(i64::MAX as u64 + 10)),
                Number(serde_json::Number::from(i64::MAX as u64 + 1)),
                Some(Number(serde_json::Number::from(9))),
            ),
            (
                // floats
                json!(5.0),
                json!(3.0),
                Some(json!(2.0)),
            ),
        ];

        for test in &integer_test_set {
            let (output, again) = _subtract(&get_inputs(test));
            assert!(again);
            assert_eq!(output, test.2);
        }
    }
}

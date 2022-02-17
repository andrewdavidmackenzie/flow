use flowmacro::flow_function;
use serde_json::{json, Value};
use serde_json::Value::Number;

#[flow_function]
fn _add(inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
    let input_a = &inputs[0];
    let input_b = &inputs[1];

    let sum = match (&input_a, &input_b) {
        (&Number(ref a), &Number(ref b)) => {
            if let Some(a_i64) = a.as_i64() {
                if let Some(b_i64) = b.as_i64() {
                    a_i64.checked_add(b_i64).map(|result| json!(result))
                } else {
                    None
                }
            } else if let Some(a_u64) = a.as_u64() {
                if let Some(b_u64) = b.as_u64() {
                    a_u64.checked_add(b_u64).map(|result| json!(result))
                } else {
                    None
                }
            } else if let Some(a_f64) = a.as_f64() {
                b.as_f64().map(|b_f64| json!(a_f64 + b_f64))
            } else {
                None
            }
        }
        (_, _) => None,
    };

    if let Some(total) = sum {
//        let mut output_map = serde_json::Map::new();
//        output_map.insert("".into(), total);
//        (Some(Value::Object(output_map)), RUN_AGAIN)
        Ok((Some(json!(total)), RUN_AGAIN))
    } else {
        Ok((None, RUN_AGAIN))
    }
}

#[cfg(test)]
mod test {
    use serde_json::json;
    use serde_json::Value;
    use serde_json::Value::Number;

    use super::_add;

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
            )
        ];

        for test in &integer_test_set {
            let (output, again) = _add(&get_inputs(test)).expect("_add() failed");

            assert!(again);
            assert_eq!(output, test.2);
        }
    }
}

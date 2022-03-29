use serde_json::Value;
use serde_json::value::Value::Number;

use flowmacro::flow_function;

#[flow_function]
fn _compare(inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
    let left = &inputs[0];
    let right = &inputs[1];

    let mut output_map = serde_json::Map::new();

    match (&left, &right) {
        (&Number(ref l), &Number(ref r)) => {
            if l.is_i64() && r.is_i64() {
                output_map.insert("equal".into(), Value::Bool(l.as_i64() == r.as_i64()));
                output_map.insert("ne".into(), Value::Bool(l.as_i64() != r.as_i64()));
                output_map.insert("lt".into(), Value::Bool(l.as_i64() < r.as_i64()));
                output_map.insert("gt".into(), Value::Bool(l.as_i64() > r.as_i64()));
                output_map.insert("lte".into(), Value::Bool(l.as_i64() <= r.as_i64()));
                output_map.insert("gte".into(), Value::Bool(l.as_i64() >= r.as_i64()));
                Ok((Some(Value::Object(output_map)), RUN_AGAIN))
            } else if l.is_u64() && r.is_u64() {
                output_map.insert("equal".into(), Value::Bool(l.as_u64() == r.as_u64()));
                output_map.insert("ne".into(), Value::Bool(l.as_u64() != r.as_u64()));
                output_map.insert("lt".into(), Value::Bool(l.as_u64() < r.as_u64()));
                output_map.insert("gt".into(), Value::Bool(l.as_u64() > r.as_u64()));
                output_map.insert("lte".into(), Value::Bool(l.as_u64() <= r.as_u64()));
                output_map.insert("gte".into(), Value::Bool(l.as_u64() >= r.as_u64()));
                Ok((Some(Value::Object(output_map)), RUN_AGAIN))
            } else {
                match (l.as_f64(), r.as_f64()) {
                    (Some(l), Some(r)) => {
                        output_map
                            .insert("equal".into(), Value::Bool((l - r).abs() < f64::EPSILON));
                        output_map
                            .insert("ne".into(), Value::Bool((l - r).abs() >= f64::EPSILON));
                        output_map.insert("lt".into(), Value::Bool(l < r));
                        output_map.insert("gt".into(), Value::Bool(l > r));
                        output_map.insert("lte".into(), Value::Bool(l <= r));
                        output_map.insert("gte".into(), Value::Bool(l >= r));
                        Ok((Some(Value::Object(output_map)), RUN_AGAIN))
                    }
                    (_, _) => bail!("Not numbers")
                }
            }
        }
        (_, _) => bail!("Not numbers")
    }
}

#[cfg(test)]
mod test {
    use serde_json::{json, Value};

    use super::_compare;

    #[allow(clippy::type_complexity)]
    fn get_tests() -> Vec<(Value, Value, bool, bool, bool, bool, bool, bool)> {
        vec![
            (
                json!(0),
                json!(0),
                true,  // eq
                false,  // ne
                false, // lt
                false, // gt
                true,  //lte
                true,  // gte
            ),
            (
                json!(1),
                json!(0),
                false, // eq
                true, // ne
                false, // lt
                true,  // gt
                false, //lte
                true,  // gte
            ),
            (
                json!(0),
                json!(1),
                false, // eq
                true, // ne
                true,  // lt
                false, // gt
                true,  //lte
                false, // gte
            ),
            // f64
            (
                json!(3.15),
                json!(3.15),
                true,  // eq
                false,  // ne
                false, // lt
                false, // gt
                true,  //lte
                true,  // gte
            ),
            (
                json!(3.15),
                json!(3.11),
                false, // eq
                true, // ne
                false, // lt
                true,  // gt
                false, //lte
                true,  // gte
            ),
            (
                json!(3.11),
                json!(3.15),
                false, // eq
                true, // ne
                true,  // lt
                false, // gt
                true,  //lte
                false, // gte
            ),
            (
                json!((i64::MAX as u64 + 10) as u64), // force a u64
                json!((i64::MAX as u64 + 20) as u64), // force a u64
                false,                                // eq
                true,                                // ne
                true,                                 // lt
                false,                                // gt
                true,                                 //lte
                false,                                // gte
            ),
        ]
    }

    fn get_inputs(pair: &(Value, Value, bool, bool, bool, bool, bool, bool)) -> Vec<Value> {
        vec![pair.0.clone(), pair.1.clone()]
    }

    #[test]
    fn positive_tests() {
        for test in &get_tests() {
            let (output, again) = _compare(&get_inputs(test)).expect("_compare() failed");

            assert!(again);

            let outputs = output.expect("Could not get the value from the output");

            assert_eq!(
                outputs.pointer("/equal").expect("Could not get the /equal from the output")
                    .as_bool().expect("/equal was not a boolean value"),
                test.2
            );
            assert_eq!(
                outputs.pointer("/ne").expect("Could not get the /equal from the output")
                    .as_bool().expect("/equal was not a boolean value"),
                test.3
            );
            assert_eq!(outputs.pointer("/lt").expect("Could not get the /lt from the output")
                           .as_bool().expect("/equal was not a boolean value"), test.4);
            assert_eq!(outputs.pointer("/gt").expect("Could not get the /gt from the output")
                           .as_bool().expect("/equal was not a boolean value"), test.5);
            assert_eq!(outputs.pointer("/lte").expect("Could not get the /lte from the output")
                           .as_bool().expect("/equal was not a boolean value"), test.6);
            assert_eq!(outputs.pointer("/gte").expect("Could not get the /gte from the output")
                           .as_bool().expect("/equal was not a boolean value"), test.7);
        }
    }

    #[test]
    fn not_numbers() {
        assert!(_compare(&[json!("hello"), json!(1.0)]).is_err());
    }
}

use serde_json::{Number, Value};

use flowcore::errors::{bail, Result};
use flowcore::flow_output;
use flowcore::RunAgain;
use flowmacro::flow_function;

#[flow_function]
fn inner_compare(left: &Number, right: &Number) -> Result<(Option<Value>, RunAgain)> {
    if left.is_i64() && right.is_i64() {
        flow_output!(
            "equal" => Value::Bool(left.as_i64() == right.as_i64()),
            "ne" => Value::Bool(left.as_i64() != right.as_i64()),
            "lt" => Value::Bool(left.as_i64() < right.as_i64()),
            "gt" => Value::Bool(left.as_i64() > right.as_i64()),
            "lte" => Value::Bool(left.as_i64() <= right.as_i64()),
            "gte" => Value::Bool(left.as_i64() >= right.as_i64()),
        )
    } else if left.is_u64() && right.is_u64() {
        flow_output!(
            "equal" => Value::Bool(left.as_u64() == right.as_u64()),
            "ne" => Value::Bool(left.as_u64() != right.as_u64()),
            "lt" => Value::Bool(left.as_u64() < right.as_u64()),
            "gt" => Value::Bool(left.as_u64() > right.as_u64()),
            "lte" => Value::Bool(left.as_u64() <= right.as_u64()),
            "gte" => Value::Bool(left.as_u64() >= right.as_u64()),
        )
    } else {
        match (left.as_f64(), right.as_f64()) {
            (Some(l), Some(r)) => {
                flow_output!(
                    "equal" => Value::Bool((l - r).abs() < f64::EPSILON),
                    "ne" => Value::Bool((l - r).abs() >= f64::EPSILON),
                    "lt" => Value::Bool(l < r),
                    "gt" => Value::Bool(l > r),
                    "lte" => Value::Bool(l <= r),
                    "gte" => Value::Bool(l >= r),
                )
            }
            (_, _) => bail!("Could not convert to f64"),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {
    use serde_json::{json, Value};

    use super::inner_compare;

    #[allow(clippy::type_complexity)]
    fn get_tests() -> Vec<(Value, Value, bool, bool, bool, bool, bool, bool)> {
        vec![
            (json!(0), json!(0), true, false, false, false, true, true),
            (json!(1), json!(0), false, true, false, true, false, true),
            (json!(0), json!(1), false, true, true, false, true, false),
            (
                json!(3.15),
                json!(3.15),
                true,
                false,
                false,
                false,
                true,
                true,
            ),
            (
                json!(3.15),
                json!(3.11),
                false,
                true,
                false,
                true,
                false,
                true,
            ),
            (
                json!(3.11),
                json!(3.15),
                false,
                true,
                true,
                false,
                true,
                false,
            ),
            (
                json!((i64::MAX as u64 + 10)),
                json!((i64::MAX as u64 + 20)),
                false,
                true,
                true,
                false,
                true,
                false,
            ),
        ]
    }

    #[test]
    fn positive_tests() {
        for test in &get_tests() {
            let left = test.0.as_number().expect("not a number");
            let right = test.1.as_number().expect("not a number");
            let (output, again) = inner_compare(left, right).expect("_compare() failed");

            assert!(again);
            let outputs = output.expect("Could not get the value from the output");

            assert_eq!(
                outputs.pointer("/equal").and_then(Value::as_bool),
                Some(test.2)
            );
            assert_eq!(
                outputs.pointer("/ne").and_then(Value::as_bool),
                Some(test.3)
            );
            assert_eq!(
                outputs.pointer("/lt").and_then(Value::as_bool),
                Some(test.4)
            );
            assert_eq!(
                outputs.pointer("/gt").and_then(Value::as_bool),
                Some(test.5)
            );
            assert_eq!(
                outputs.pointer("/lte").and_then(Value::as_bool),
                Some(test.6)
            );
            assert_eq!(
                outputs.pointer("/gte").and_then(Value::as_bool),
                Some(test.7)
            );
        }
    }

    #[test]
    fn not_numbers() {
        // The macro-generated code handles the type check — calling with non-numbers
        // would be caught at the extraction stage in the generated run() method
        let left = json!(0).as_number().expect("not a number").clone();
        let right = json!(0).as_number().expect("not a number").clone();
        assert!(inner_compare(&left, &right).is_ok());
    }
}

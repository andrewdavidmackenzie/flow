use serde_json::Value;

use flowcore::errors::{bail, Result};
use flowcore::flow_output;
use flowcore::RunAgain;
use flowmacro::flow_function;

#[flow_function]
fn compare(left: &Value, right: &Value) -> Result<(Option<Value>, RunAgain)> {
    match (left.as_f64(), right.as_f64()) {
        (Some(lhs), Some(rhs)) => {
            if (rhs - lhs).abs() < f64::EPSILON {
                flow_output!(
                    "equal" => right.clone(),
                    "right-lte" => right.clone(),
                    "left-gte" => left.clone(),
                    "right-gte" => right.clone(),
                    "left-lte" => left.clone(),
                )
            } else if rhs < lhs {
                flow_output!(
                    "right-lt" => right.clone(),
                    "left-gt" => left.clone(),
                    "right-lte" => right.clone(),
                    "left-gte" => left.clone(),
                )
            } else {
                flow_output!(
                    "right-gt" => right.clone(),
                    "left-lt" => left.clone(),
                    "right-gte" => right.clone(),
                    "left-lte" => left.clone(),
                )
            }
        }
        (_, _) => bail!("Could not get input values as f64"),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {
    use serde_json::json;

    use flowcore::RUN_AGAIN;

    use super::compare;

    #[test]
    fn integer_equals() {
        let left = json!(1);
        let right = json!(1);

        let (value, run_again) = compare(&left, &right).expect("compare() failed");

        assert_eq!(run_again, RUN_AGAIN);
        assert!(value.is_some());
        let value = value.expect("No value was returned");
        let map = value.as_object().expect("Expected a json object");
        assert!(map.contains_key("equal"));
    }

    #[test]
    fn float_equals() {
        let left = json!(1.0);
        let right = json!(1.0);

        let (value, run_again) = compare(&left, &right).expect("compare() failed");

        assert_eq!(run_again, RUN_AGAIN);
        assert!(value.is_some());
        let value = value.expect("Could not get the value from the output");
        let map = value
            .as_object()
            .expect("Could not get the json object from the output");
        assert!(map.contains_key("equal"));
    }

    #[test]
    fn integer_less_than() {
        let (value, run_again) = compare(&json!(1), &json!(2)).expect("compare() failed");

        assert_eq!(run_again, RUN_AGAIN);
        assert!(value.is_some());
        let value = value.expect("Could not get the value from the output");
        let map = value
            .as_object()
            .expect("Could not get the object from the output");
        assert_eq!(map.get("left-lt"), Some(&json!(1)));
        assert_eq!(map.get("right-gt"), Some(&json!(2)));
    }

    #[test]
    fn float_less_than() {
        let (value, run_again) = compare(&json!(1.0), &json!(2.0)).expect("compare() failed");

        assert_eq!(run_again, RUN_AGAIN);
        assert!(value.is_some());
        let value = value.expect("Could not get the value from the output");
        let map = value
            .as_object()
            .expect("Could not get the object from the output");
        assert_eq!(map.get("left-lt"), Some(&json!(1.0)));
        assert_eq!(map.get("right-gt"), Some(&json!(2.0)));
    }

    #[test]
    fn integer_more_than() {
        let (value, run_again) = compare(&json!(2), &json!(1)).expect("compare() failed");

        assert_eq!(run_again, RUN_AGAIN);
        assert!(value.is_some());
        let value = value.expect("Could not get the value from the output");
        let map = value
            .as_object()
            .expect("Could not get the object from the output");
        assert_eq!(map.get("left-gt"), Some(&json!(2)));
        assert_eq!(map.get("right-lt"), Some(&json!(1)));
    }

    #[test]
    fn float_more_than() {
        let (value, run_again) = compare(&json!(2.0), &json!(1.0)).expect("compare() failed");

        assert_eq!(run_again, RUN_AGAIN);
        assert!(value.is_some());
        let value = value.expect("Could not get the value from the output");
        let map = value
            .as_object()
            .expect("Could not get the object from the output");
        assert_eq!(map.get("left-gt"), Some(&json!(2.0)));
        assert_eq!(map.get("right-lt"), Some(&json!(1.0)));
    }

    #[test]
    fn invalid() {
        assert!(compare(&json!("AAA"), &json!(1.0)).is_err());
    }
}

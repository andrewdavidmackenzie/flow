use flow_macro::flow;
use flowcore::{RUN_AGAIN, RunAgain};
use serde_json::Value;

#[flow(definition = "compare_switch.toml")]
struct foo {}

impl Implementation for CompareSwitch {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain) {
        let left = &inputs[0];
        let right = &inputs[1];
        match (left.as_f64(), right.as_f64()) {
            (Some(lhs), Some(rhs)) => {
                let mut output_map = serde_json::Map::new();
                if (rhs - lhs).abs() < f64::EPSILON {
                    output_map.insert("equal".into(), right.clone());
                    output_map.insert("right-lte".into(), right.clone());
                    output_map.insert("left-gte".into(), left.clone());
                    output_map.insert("right-gte".into(), right.clone());
                    output_map.insert("left-lte".into(), left.clone());
                } else if rhs < lhs {
                    output_map.insert("right-lt".into(), right.clone());
                    output_map.insert("left-gt".into(), left.clone());
                    output_map.insert("right-lte".into(), right.clone());
                    output_map.insert("left-gte".into(), left.clone());
                } else if rhs > lhs {
                    output_map.insert("right-gt".into(), right.clone());
                    output_map.insert("left-lt".into(), left.clone());
                    output_map.insert("right-gte".into(), right.clone());
                    output_map.insert("left-lte".into(), left.clone());
                }

                let output = Value::Object(output_map);

                (Some(output), RUN_AGAIN)
            }
            (_, _) => (None, RUN_AGAIN),
        }
    }
}

#[cfg(test)]
mod test {
    use flowcore::{Implementation, RUN_AGAIN};
    use serde_json::json;

    use super::CompareSwitch;

    #[test]
    fn integer_equals() {
        let left = json!(1);
        let right = json!(1);
        let inputs = vec![left, right];

        let comparer = &CompareSwitch {} as &dyn Implementation;

        let (value, run_again) = comparer.run(&inputs);

        assert_eq!(run_again, RUN_AGAIN);
        assert!(value.is_some());
        let value = value.expect("No value was returned");
        let map = value.as_object().expect("Expected a Map json object");
        assert!(map.contains_key("equal"));
    }

    #[test]
    fn float_equals() {
        let left = json!(1.0);
        let right = json!(1.0);
        let inputs = vec![left, right];

        let comparer = &CompareSwitch {} as &dyn Implementation;

        let (value, run_again) = comparer.run(&inputs);

        assert_eq!(run_again, RUN_AGAIN);
        assert!(value.is_some());
        let value = value.expect("Could not get the value from the output");
        let map = value.as_object().expect("Could not get the json object from the output");
        assert!(map.contains_key("equal"));
    }

    #[test]
    fn integer_less_than() {
        let inputs = vec![json!(1), json!(2)];
        let comparer = &CompareSwitch {} as &dyn Implementation;
        let (value, run_again) = comparer.run(&inputs);

        assert_eq!(run_again, RUN_AGAIN);
        assert!(value.is_some());
        let value = value.expect("Could not get the value from the output");
        let map = value.as_object().expect("Could not get the Map json object from the output");
        assert_eq!(map.get("left-lt"), Some(&json!(1)));
        assert_eq!(map.get("right-gt"), Some(&json!(2)));
    }

    #[test]
    fn float_less_than() {
        let inputs = vec![json!(1.0), json!(2.0)];
        let comparer = &CompareSwitch {} as &dyn Implementation;
        let (value, run_again) = comparer.run(&inputs);

        assert_eq!(run_again, RUN_AGAIN);
        assert!(value.is_some());
        let value = value.expect("Could not get the value from the output");
        let map = value.as_object().expect("Could not get the Map json object from the output");
        assert_eq!(map.get("left-lt"), Some(&json!(1.0)));
        assert_eq!(map.get("right-gt"), Some(&json!(2.0)));
    }

    #[test]
    fn integer_more_than() {
        let inputs = vec![json!(2), json!(1)];
        let comparer = &CompareSwitch {} as &dyn Implementation;
        let (value, run_again) = comparer.run(&inputs);

        assert_eq!(run_again, RUN_AGAIN);
        assert!(value.is_some());
        let value = value.expect("Could not get the value from the output");
        let map = value.as_object().expect("Could not get the Map json object from the output");
        assert_eq!(map.get("left-gt"), Some(&json!(2)));
        assert_eq!(map.get("right-lt"), Some(&json!(1)));
    }

    #[test]
    fn float_more_than() {
        let inputs = vec![json!(2.0), json!(1.0)];
        let comparer = &CompareSwitch {} as &dyn Implementation;
        let (value, run_again) = comparer.run(&inputs);

        assert_eq!(run_again, RUN_AGAIN);
        assert!(value.is_some());
        let value = value.expect("Could not get the value from the output");
        let map = value.as_object().expect("Could not get the Map json object from the output");
        assert_eq!(map.get("left-gt"), Some(&json!(2.0)));
        assert_eq!(map.get("right-lt"), Some(&json!(1.0)));
    }

    #[test]
    fn invalid() {
        let inputs = vec![json!("AAA"), json!(1.0)];
        let comparer = &CompareSwitch {} as &dyn Implementation;
        let (value, run_again) = comparer.run(&inputs);

        assert_eq!(run_again, RUN_AGAIN);
        assert!(value.is_none());
    }
}

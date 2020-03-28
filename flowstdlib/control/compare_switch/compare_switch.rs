use flow_impl::{Implementation, RUN_AGAIN, RunAgain};
use flow_impl_derive::FlowImpl;
use serde_json::Value;

#[derive(FlowImpl)]
/// Compare two input values and output different the right hand value at different output route
/// corresponding to is equal, greater than, greater than or equal, less than or less than or equal.
///
/// ## Include using
/// ```toml
/// [[process]]
/// alias = "compare_switch"
/// source = "lib://flowstdlib/control/compare_switch"
/// ```
///
/// ## Inputs
/// * `left` - left hand input
/// * `right` - right hand input
///
/// ## Outputs
/// * `equal` - outputs right hand value if the two values are equal
/// * `lt` - outputs right hand value if the left hand value is less than the right hand value
/// * `lte` - outputs right hand value if the left hand value is less than or equal to the right hand value
/// * `gt` - outputs right hand value if the left hand value is greater than the right hand value
/// * `gte` - outputs right hand value if the left hand value is greater than or equal to the right hand value
#[derive(Debug)]
pub struct CompareSwitch;

impl Implementation for CompareSwitch {
    fn run(&self, inputs: &Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        let left = &inputs[0][0];
        let right = &inputs[1][0];
        match (left.as_f64(), right.as_f64()) {
            (Some(lhs), Some(rhs)) => {
                let mut output_map = serde_json::Map::new();
                if rhs == lhs {
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
                } else  if rhs > lhs {
                    output_map.insert("right-gt".into(), right.clone());
                    output_map.insert("left-lt".into(), left.clone());
                    output_map.insert("right-gte".into(), right.clone());
                    output_map.insert("left-lte".into(), left.clone());
                }

                let output = Value::Object(output_map);

                (Some(output), RUN_AGAIN)
            }
            (_, _) => {
                println!("Unsupported types in compare_switch");
                (None, RUN_AGAIN)
            }
        }
    }
}

#[cfg(test)]
mod test {
    use flow_impl::{Implementation, RUN_AGAIN};
    use serde_json::json;

    use super::CompareSwitch;

    #[test]
    fn integer_equals() {
        let left = vec!(json!(1));
        let right = vec!(json!(1));
        let inputs = vec!(left, right);

        let comparer = &CompareSwitch{} as &dyn Implementation;

        let (value, run_again) = comparer.run(&inputs);

        assert_eq!(run_again, RUN_AGAIN);
        assert!(value.is_some());
        let value = value.unwrap();
        let map = value.as_object().unwrap();
        assert!(map.contains_key("equal"));
    }

    #[test]
    fn float_equals() {
        let left = vec!(json!(1.0));
        let right = vec!(json!(1.0));
        let inputs = vec!(left, right);

        let comparer = &CompareSwitch{} as &dyn Implementation;

        let (value, run_again) = comparer.run(&inputs);

        assert_eq!(run_again, RUN_AGAIN);
        assert!(value.is_some());
        let value = value.unwrap();
        let map = value.as_object().unwrap();
        assert!(map.contains_key("equal"));
    }
}
use flow_impl::{Implementation, RUN_AGAIN, RunAgain};
use flow_impl_derive::FlowImpl;
use serde_json::Value;

#[derive(FlowImpl)]
/// Compare two input values and output different the right hand value at different output route
/// corresponding to is equal, greater than, greater than or equal, less than or less than or equal.
#[derive(Debug)]
pub struct CompareSwitch;

impl Implementation for CompareSwitch {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain) {
        let left = &inputs[0];
        let right = &inputs[1];
        match (left.as_f64(), right.as_f64()) {
            (Some(lhs), Some(rhs)) => {
                let mut output_map = serde_json::Map::new();
                if (rhs - lhs).abs() < std::f64::EPSILON {
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
                println!("Unsupported input types in 'compare_switch': {:?}", inputs);
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
        let left = json!(1);
        let right = json!(1);
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
        let left = json!(1.0);
        let right = json!(1.0);
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
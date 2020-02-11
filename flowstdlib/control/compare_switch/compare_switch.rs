use flow_impl::{Implementation, RUN_AGAIN, RunAgain};
use flow_impl_derive::FlowImpl;
use serde_json::Value;

#[derive(FlowImpl)]
/// Compare two input values and output different the right hand value at different output routes
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
pub struct CompareSwitch;

impl Implementation for CompareSwitch {
    fn run(&self, mut inputs: Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        match (inputs[0].remove(0).as_i64(), inputs[1].remove(0).as_i64()) {
            (Some(left), Some(right)) => {
                let mut output_map = serde_json::Map::new();
                if left == right {
                    output_map.insert("equal".into(), Value::Number(serde_json::Number::from(right)));
                }

                if left < right {
                    output_map.insert("lt".into(), Value::Number(serde_json::Number::from(right)));
                }

                if left > right {
                    output_map.insert("gt".into(), Value::Number(serde_json::Number::from(right)));
                }

                if left <= right {
                    output_map.insert("lte".into(), Value::Number(serde_json::Number::from(right)));
                }

                if left >= right {
                    output_map.insert("gte".into(), Value::Number(serde_json::Number::from(right)));
                }

                let output = Value::Object(output_map);

                (Some(output), RUN_AGAIN)
            }
            (_, _) => (None, RUN_AGAIN)
        }
    }
}
use flow_impl::{Implementation, RUN_AGAIN, RunAgain};
use flow_impl_derive::FlowImpl;
use serde_json::Value;

#[derive(FlowImpl)]
/// Compare two input values and output different boolean values depending on if the comparison
/// is equal, greater than, greater than or equal, less than or less than or equal.
///
/// ## Include using
/// ```toml
/// [[process]]
/// alias = "compare"
/// source = "lib://flowstdlib/math/compare"
/// ```
///
/// ## Inputs
/// * `left` - left hand input
/// * `right` - right hand input
///
/// ## Outputs
/// * `equal` \[Boolean\] - outputs true if the two values are equal
/// * `lt` \[Boolean\] - outputs true if the left hand value is less than the right hand value
/// * `lte` \[Boolean\] - outputs true if the left hand value is less than or equal to the right hand value
/// * `gt` \[Boolean\] - outputs true if the left hand value is greater than the right hand value
/// * `gte` \[Boolean\] - outputs true if the left hand value is greater than or equal to the right hand value
#[derive(Debug)]
pub struct Compare;

impl Implementation for Compare {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain) {
        match (inputs[0].as_f64(), inputs[1].as_f64()) {
            (Some(left), Some(right)) => {
                let mut output_map = serde_json::Map::new();
                output_map.insert("equal".into(), Value::Bool((left - right).abs() < std::f64::EPSILON));
                output_map.insert("lt".into(), Value::Bool(left < right));
                output_map.insert("gt".into(), Value::Bool(left > right));
                output_map.insert("lte".into(), Value::Bool(left <= right));
                output_map.insert("gte".into(), Value::Bool(left >= right));
                let output = Value::Object(output_map);

                (Some(output), RUN_AGAIN)
            }
            (_, _) => (None, RUN_AGAIN)
        }
    }
}
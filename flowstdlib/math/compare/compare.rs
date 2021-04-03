use serde_json::Value;

use flow_impl_derive::FlowImpl;
use flowcore::{Implementation, RUN_AGAIN, RunAgain};

#[derive(FlowImpl)]
/// Compare two input values and output different boolean values depending on if the comparison
/// is equal, greater than, greater than or equal, less than or less than or equal.
#[derive(Debug)]
pub struct Compare;

impl Implementation for Compare {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain) {
        match (inputs[0].as_f64(), inputs[1].as_f64()) {
            (Some(left), Some(right)) => {
                let mut output_map = serde_json::Map::new();
                output_map.insert(
                    "equal".into(),
                    Value::Bool((left - right).abs() < std::f64::EPSILON),
                );
                output_map.insert("lt".into(), Value::Bool(left < right));
                output_map.insert("gt".into(), Value::Bool(left > right));
                output_map.insert("lte".into(), Value::Bool(left <= right));
                output_map.insert("gte".into(), Value::Bool(left >= right));
                let output = Value::Object(output_map);

                (Some(output), RUN_AGAIN)
            }
            (_, _) => (None, RUN_AGAIN),
        }
    }
}

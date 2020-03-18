use flow_impl::{Implementation, RUN_AGAIN, RunAgain};
use flow_impl_derive::FlowImpl;
use serde_json::Value;

#[derive(FlowImpl)]
/// Accumulate input values into an array upto the limit specified
///
/// ## Include using
/// ```toml
/// [[process]]
/// alias = "accumulate"
/// source = "lib://flowstdlib/data/accumulate"
/// ```
///
/// ## Input
/// name = "values"
///
/// ## Input
/// name = "accumulated"
///
/// ## Input
/// name = "limit"
///
/// ## Outputs
/// * type Array of Number
#[derive(Debug)]
pub struct Accumulate;

impl Implementation for Accumulate {
    fn run(&self, mut inputs: Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        let mut values = inputs.remove(0);
        let mut accumulated = inputs.remove(0);
        let limit = inputs.remove(0).remove(0);
        accumulated.append(&mut values);

        let mut output_map = serde_json::Map::new();

        if accumulated.len() > limit.as_u64().unwrap() as usize {
            output_map.insert("chunk".into(), Value::Array(accumulated));
        } else {
            output_map.insert("partial".into(), Value::Array(accumulated));
        }

        output_map.insert("limit".into(), limit);

        let output = Value::Object(output_map);

        (Some(output), RUN_AGAIN)
    }
}
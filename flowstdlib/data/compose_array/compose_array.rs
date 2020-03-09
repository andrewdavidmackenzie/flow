use flow_impl::{Implementation, RUN_AGAIN, RunAgain};
use flow_impl_derive::FlowImpl;
use serde_json::Value;

#[derive(FlowImpl)]
/// Take 'N' input values (width='N') from the input stream and gather them into a single output item,
/// which is an array of 'N' items long.
///
/// ## Include using
/// ```toml
/// [[process]]
/// alias = "compose_array"
/// source = "lib://flowstdlib/data/compose_array/compose_array"
/// ```
///
/// ## Input
/// * type Number
///
/// ## Outputs
/// * type Array of Number (Array/Number)
pub struct ComposeArray;

impl Implementation for ComposeArray {
    fn run(&self, mut inputs: Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        let output_vec = inputs.remove(0);
        let output = Value::Array(output_vec);

        (Some(output), RUN_AGAIN)
    }
}
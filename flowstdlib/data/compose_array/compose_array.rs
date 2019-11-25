#[cfg(target_arch = "wasm32")]
#[macro_use]
extern crate serde_json;

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
        let mut input_stream = inputs.remove(0);
        let mut output_vec = Vec::new();

        output_vec.push(input_stream.remove(0));
        output_vec.push(input_stream.remove(0));
        output_vec.push(input_stream.remove(0));
        output_vec.push(input_stream.remove(0));

        let output = Value::Array(output_vec);

        (Some(output), RUN_AGAIN)
    }
}
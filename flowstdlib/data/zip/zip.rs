use flow_impl::{Implementation, RUN_AGAIN, RunAgain};
use flow_impl_derive::FlowImpl;
use serde_json::json;
use serde_json::Value;

#[derive(FlowImpl)]
/// Takes two arrays of values and produce an array of tuples of pairs of values from each input array.
///
/// ## Include using
/// ```toml
/// [[process]]
/// alias = "zip"
/// source = "lib://flowstdlib/data/zip"
/// ```
///
/// ## Input
/// * left - the 'left' array
/// * right - the 'right' array
///
/// ## Outputs
/// * tuples - the array of tuples of (left, right)
pub struct Zip;

impl Implementation for Zip {
    fn run(&self, mut inputs: Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        let left = inputs.remove(0);
        let right = inputs.remove(0);

        let tuples = left.iter().zip(right.iter());

        let tuples_vec: Vec<(&Value, &Value)> = tuples.collect();
        (Some(json!(tuples_vec)), RUN_AGAIN)
    }
}
use flow_impl::{Implementation, RUN_AGAIN, RunAgain};
use flow_impl_derive::FlowImpl;
use serde_json::Value;

#[derive(FlowImpl)]
/// Remove a value from a vector of values
///
/// ## Include using
/// ```toml
/// [[process]]
/// alias = "remove"
/// source = "lib://flowstdlib/data/remove"
/// ```
///
/// ## Input
/// name = "value"
/// * The value to remove from the array
///
/// ## Input
/// name = "array"
/// type = "Array"
/// * An array, to remove `value` from
///
/// ## Outputs
/// type = "Array"
/// * The resulting array
#[derive(Debug)]
pub struct Remove;

impl Implementation for Remove {
    fn run(&self, mut inputs: Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        // Inputs
        let value = inputs.remove(0).remove(0);
        let mut input1 = inputs.remove(0).remove(0);
        let array = input1.as_array_mut().unwrap();

        // Operation
        array.retain(|val| val != &value);

        // Output
        let output = Value::Array(array.clone());

        (Some(output), RUN_AGAIN)
    }
}

#[cfg(test)]
mod test {
    fn remove_1() {

    }
}
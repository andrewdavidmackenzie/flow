use flow_impl::{Implementation, RUN_AGAIN, RunAgain};
use flow_impl_derive::FlowImpl;
use serde_json::Value;
use serde_json::Value::Number;

#[derive(FlowImpl)]
/// Subtract one input from another to produce a new output
///
/// ## Include using
/// ```toml
/// [[process]]
/// alias = "subtract"
/// source = "lib://flowstdlib/math/subtract"
/// ```
///
/// ## Inputs
/// * `i1` - first input of type `Number`
/// * `i2` - second input of type `Number`
///
/// ## Outputs
/// * `i1` minus `i2` of type `Number`
#[derive(Debug)]
pub struct Subtract;

impl Implementation for Subtract {
    fn run(&self, inputs: &Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        let input_a = &inputs[0];
        let input_b = &inputs[1];
        let mut value = None;

        match (&input_a[0], &input_b[0]) {
            (&Number(ref a), &Number(ref b)) => {
                // TODO mixed signed and unsigned integers
                if a.is_i64() && b.is_i64() {
                    value = Some(Value::Number(serde_json::Number::from(a.as_i64().unwrap() - b.as_i64().unwrap())));
                } else if a.is_u64() && b.is_u64() {
                    value = Some(Value::Number(serde_json::Number::from(a.as_u64().unwrap() - b.as_u64().unwrap())));
                } else if a.is_f64() && b.is_f64() {
                    value = Some(Value::Number(serde_json::Number::from_f64(a.as_f64().unwrap() - b.as_f64().unwrap()).unwrap()));
                }
            }
            (_, _) => {}
        }

        (value, RUN_AGAIN)
    }
}
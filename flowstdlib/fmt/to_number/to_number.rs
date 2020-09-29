use flow_impl::{Implementation, RUN_AGAIN, RunAgain};
use flow_impl_derive::FlowImpl;
use serde_json::json;
use serde_json::Value;

#[derive(FlowImpl)]
/// Convert a String to a number
///
/// ## Include using
/// ```toml
/// [[process]]
/// alias = "to_number"
/// source = "lib://flowstdlib/fmt/to_number"
/// ```
///
/// ## Input
/// * The String to convert
///
/// ## Output
/// * The Number equivalent of String input if possible
#[derive(Debug)]
pub struct ToNumber;

impl Implementation for ToNumber {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain) {
        let mut value = None;
        let input = &inputs[0];

        if input.is_null() {
            (Some(Value::Null), RUN_AGAIN)
        } else if let Value::String(string) = input {
            if let Ok(number) = string.parse::<i64>() {
                let number = json!(number);
                value = Some(number);
            } else if let Ok(number) = string.parse::<f64>() {
                let number = Value::Number(serde_json::Number::from_f64(number).unwrap());
                value = Some(number);
            }
            (value, RUN_AGAIN)
        } else {
            (None, RUN_AGAIN)
        }
    }
}
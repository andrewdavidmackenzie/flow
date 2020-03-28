use flow_impl::{Implementation, RUN_AGAIN, RunAgain};
use flow_impl_derive::FlowImpl;
use serde_json::Value;

#[derive(FlowImpl)]
/// Convert an input type to a String
///
/// ## Include using
/// ```toml
/// [[process]]
/// alias = "to_string"
/// source = "lib://flowstdlib/fmt/to_string"
/// ```
///
/// ## Input
/// * The data to convert to a String. Current types supported are:
/// * String - a bit redundant, but it works
/// * Bool - Boolean JSON value
/// * Number - A JSON Number
/// * Array - An JSON array of values that can be converted, they are converted one by one
///
/// ## Output
/// * The String equivalent of the input value
#[derive(Debug)]
pub struct ToString;

impl Implementation for ToString {
    fn run(&self, inputs: &Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        let input = &inputs[0][0];
        match input {
            Value::String(_) => (Some(input.clone()), RUN_AGAIN),
            Value::Bool(boolean) => (Some(Value::String(boolean.to_string())), RUN_AGAIN),
            Value::Number(number) => (Some(Value::String(number.to_string())), RUN_AGAIN),
            _ => (None, RUN_AGAIN)
        }
    }
}
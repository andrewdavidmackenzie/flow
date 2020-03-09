use flow_impl::{Implementation, RUN_AGAIN, RunAgain};
use flow_impl_derive::FlowImpl;
use serde_json::json;
use serde_json::Value;
use serde_json::Value::String as JsonString;

#[derive(FlowImpl)]
/// Reverse a String
///
/// ## Include using
/// ```toml
/// [[process]]
/// alias = "reverse"
/// source = "lib://flowstdlib/fmt/reverse"
/// ```
///
/// ## Input
/// * The String to reverse
///
/// ## Output
/// * "original" - The original input string
/// * "reversed" - The input string reversed
#[derive(Debug)]
pub struct Reverse;

impl Implementation for Reverse {
    fn run(&self, mut inputs: Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        let mut value = None;

        let input = inputs.remove(0).remove(0);
        match input {
            JsonString(ref s) => {
                value = Some(json!({
                    "reversed" : s.chars().rev().collect::<String>(),
                    "original": s
                }));
            }
            _ => {}
        }

        (value, RUN_AGAIN)
    }
}
use flow_impl_derive::FlowImpl;
use flowcore::Implementation;
use serde_json::json;
use serde_json::Value;
use serde_json::Value::String as JsonString;

#[derive(FlowImpl, Debug)]
pub struct Reverser;

impl Implementation for Reverser {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, bool) {
        let mut value = None;

        let input = &inputs[0];
        if let JsonString(ref s) = input {
            value = Some(json!({
                "reversed" : s.chars().rev().collect::<String>(),
                "original": s
            }));
        }

        (value, true)
    }
}

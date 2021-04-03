use serde_json::Value;

use flow_impl_derive::FlowImpl;
use flowcore::{Implementation, RUN_AGAIN, RunAgain};

#[derive(FlowImpl)]
/// Route data to one or another based on a boolean control value.
#[derive(Debug)]
pub struct Route;

impl Implementation for Route {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain) {
        let data = &inputs[0];
        let control = &inputs[1].as_bool().unwrap();

        let mut output_map = serde_json::Map::new();
        if *control {
            output_map.insert("true".into(), data.clone());
        } else {
            output_map.insert("false".into(), data.clone());
        }

        (Some(Value::Object(output_map)), RUN_AGAIN)
    }
}

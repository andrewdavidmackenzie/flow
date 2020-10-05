use flow_impl::{Implementation, RUN_AGAIN, RunAgain};
use flow_impl_derive::FlowImpl;
use serde_json::Value;

#[derive(FlowImpl)]
/// Select which data to output, based on a boolean control value.
#[derive(Debug)]
pub struct Select;

impl Implementation for Select {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain) {
        let i1 = &inputs[0];
        let i2 = &inputs[1];
        let control = &inputs[2].as_bool().unwrap();

        let mut output_map = serde_json::Map::new();
        if *control {
            output_map.insert("select_i1".into(), i1.clone());
            output_map.insert("select_i2".into(), i2.clone());
        } else {
            output_map.insert("select_i1".into(), i2.clone());
            output_map.insert("select_i2".into(), i1.clone());
        }

        (Some(Value::Object(output_map)), RUN_AGAIN)
    }
}
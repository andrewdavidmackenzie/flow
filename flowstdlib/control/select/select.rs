use flow_impl::{Implementation, RUN_AGAIN, RunAgain};
use flow_impl_derive::FlowImpl;
use serde_json::Value;

#[derive(FlowImpl)]
/// Route data to one or another based on a boolean control value.
///
/// ## Include using
/// ```toml
/// [[process]]
/// alias = "select"
/// source = "lib://flowstdlib/control/select"
/// ```
///
/// ## Inputs
/// * `i1` - input i1
/// * `i2` - input i2
/// * `control` - a boolean value to selection of inputs passed to outputs
///
/// ## Outputs
/// * `select_i1` if `control` is true `i1` is routed here else `i2` is routed here
/// * `select_i2` if `control` is true `i2` is routed here else `i1` is routed here
#[derive(Debug)]
pub struct Select;

impl Implementation for Select {
    fn run(&self, mut inputs: Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        let i1 = inputs[0].remove(0);
        let i2 = inputs[1].remove(0);
        let control = inputs[2].remove(0).as_bool().unwrap();

        let mut output_map = serde_json::Map::new();
        if control {
            output_map.insert("select_i1".into(), i1);
            output_map.insert("select_i2".into(), i2);
        } else {
            output_map.insert("select_i1".into(), i2);
            output_map.insert("select_i2".into(), i1);
        }

        (Some(Value::Object(output_map)), RUN_AGAIN)
    }
}
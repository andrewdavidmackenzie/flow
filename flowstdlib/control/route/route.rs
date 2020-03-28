use flow_impl::{Implementation, RUN_AGAIN, RunAgain};
use flow_impl_derive::FlowImpl;
use serde_json::Value;

#[derive(FlowImpl)]
/// Route data to one or another based on a boolean control value.
///
/// ## Include using
/// ```toml
/// [[process]]
/// alias = "route"
/// source = "lib://flowstdlib/control/route"
/// ```
///
/// ## Inputs
/// * `data` - the data flow we wish to control the flow if
/// * `control` - a boolean value to determine which output roue `data` is passed to
///
/// ## Outputs
/// * `true` if `control` is true `data` is routed here
/// * `false` if `control` is false `data` is routed here
#[derive(Debug)]
pub struct Route;

impl Implementation for Route {
    fn run(&self, inputs: &Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        let data = &inputs[0][0];
        let control = &inputs[1][0].as_bool().unwrap();

        let mut output_map = serde_json::Map::new();
        if *control {
            output_map.insert("true".into(), data.clone());
        } else {
            output_map.insert("false".into(), data.clone());
        }

        (Some(Value::Object(output_map)), RUN_AGAIN)
    }
}
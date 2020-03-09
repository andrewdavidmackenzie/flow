use flow_impl::{Implementation, RUN_AGAIN, RunAgain};
use flow_impl_derive::FlowImpl;
use serde_json::Value;

#[derive(FlowImpl)]
/// Control the flow of data (flow or disapear it) based on a boolean control value.
///
/// ## Include using
/// ```toml
/// [[process]]
/// alias = "tap"
/// source = "lib://flowstdlib/control/tap"
/// ```
///
/// ## Inputs
/// * `data` - the data flow we wish to control the flow if
/// * `control` - a boolean value to determine in `data` is passed on or not
///
/// ## Outputs
/// * `data` if `control` is true, nothing if `control` is false
#[derive(Debug)]
pub struct Tap;

impl Implementation for Tap {
    fn run(&self, mut inputs: Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        let mut value = None;
        let data = inputs[0].remove(0);
        let control = inputs[1].remove(0).as_bool().unwrap();
        if control {
            value = Some(data);
        }

        (value, RUN_AGAIN)
    }
}
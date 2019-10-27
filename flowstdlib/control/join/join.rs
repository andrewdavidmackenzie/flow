extern crate core;
extern crate flow_impl;
extern crate flow_impl_derive;

use flow_impl::{Implementation, RUN_AGAIN, RunAgain};
use flow_impl_derive::FlowImpl;
use serde_json::Value;

#[derive(FlowImpl)]
/// Control the flow of a piece of data by waiting for a second value to be available
///
/// ## Include using
/// ```toml
/// [[process]]
/// alias = "join"
/// source = "lib://flowstdlib/control/join"
/// ```
///
/// ## Inputs
/// * `data` - the data we wish to control the flow of
/// * `control` - a second value we wait on
///
/// ## Outputs
/// * `data`
pub struct Join;

impl Implementation for Join {
    fn run(&self, mut inputs: Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        let data = Some(inputs[0].remove(0));

        (data, RUN_AGAIN)
    }
}
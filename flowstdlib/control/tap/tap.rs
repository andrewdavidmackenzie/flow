extern crate core;
extern crate flow_impl_derive;
extern crate flowrlib;

use flow_impl_derive::FlowImpl;
use flowrlib::implementation::{Implementation, RUN_AGAIN, RunAgain};
use serde_json::Value;

#[derive(FlowImpl)]
pub struct Tap;

/*
    A control switch function that outputs the "data" input IF the "control" input is true,
    otherwise it does not produce any output
*/
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
use serde_json::Value;

use flow_impl::implementation::{Implementation, RUN_AGAIN, RunAgain};

pub struct Join;

/*
    A function that outputs the "data" input once the second input "control" is available and
    the function can run
*/
impl Implementation for Join {
    fn run(&self, mut inputs: Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        let data = Some(inputs[0].remove(0));

        (data, RUN_AGAIN)
    }
}
use flow_impl::implementation::{Implementation, RUN_AGAIN, RunAgain};
use serde_json::Value;

pub struct Readline;

impl Implementation for Readline {
    fn run(&self, _inputs: Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        // TODO convert to ide sys
        (Some(json!("readline string")), RUN_AGAIN)
    }
}
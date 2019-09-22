use flowrlib::implementation::{Implementation, RUN_AGAIN, RunAgain};
use serde_json::Value;

pub struct Stdin;

impl Implementation for Stdin {
    fn run(&self, mut _inputs: Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        // TODO convert to ide sys
        (Some(json!("stdin string")), RUN_AGAIN)
    }
}
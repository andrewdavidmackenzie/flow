use flowcore::{Implementation, RunAgain, RUN_AGAIN};
use serde_json::{json, Value};

/// `Implementation` struct for the `Stdout` function
pub struct Stdout {}

impl Implementation for Stdout {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain) {
        let mut output = None;

        if inputs.len() == 1 {
            let input = &inputs[0];
            output = Some(json!(input.to_string()));
        }

        (output, RUN_AGAIN)
    }
}

use flowcore::{Implementation, RUN_AGAIN, RunAgain};
use flowcore::errors::*;
use serde_json::{json, Value};

/// `Implementation` struct for the `Test` function
pub struct Test {}

impl Implementation for Test {
    fn run(&self, inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
        if inputs.len() != 1 {
            bail!("Incorrect number of inputs")
        }

        let input = &inputs[0];
        let output = Some(json!(input.to_string()));

        Ok((output, RUN_AGAIN))
    }
}

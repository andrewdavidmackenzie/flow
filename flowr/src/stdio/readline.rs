use std::io::{self};

use serde_json::Value;

use flowrlib::implementation::{DONT_RUN_AGAIN, Implementation, RunAgain};

#[derive(Debug)]
pub struct Readline;

impl Implementation for Readline {
    fn run(&self, _inputs: Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        let mut input = String::new();

        match io::stdin().read_line(&mut input) {
            Ok(n) if n > 0 => {
                let value = Value::String(input.trim().to_string());
                return (Some(value), true);
            }
            _ => {}
        }

        (None, DONT_RUN_AGAIN)
    }
}
use std::io::{self, Read};

use serde_json::Value;

use flowrlib::implementation::{DONT_RUN_AGAIN, Implementation, RunAgain};

#[derive(Debug)]
pub struct Stdin;

impl Implementation for Stdin {
    fn run(&self, mut _inputs: Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        let mut value = None;

        let mut buffer = String::new();
        let stdin = io::stdin();
        let mut handle = stdin.lock();
        if let Ok(size) = handle.read_to_string(&mut buffer) {
            if size > 0 {
                let input = Value::String(buffer.trim().to_string());
                value = Some(input);
            }
        }

        (value, DONT_RUN_AGAIN)
    }
}
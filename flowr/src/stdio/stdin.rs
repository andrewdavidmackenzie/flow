use std::io::{self, Read};

use flow_impl::{DONT_RUN_AGAIN, Implementation, RunAgain};
use serde_json::Value;

#[derive(Debug)]
/// `Implementation` struct for the `stdin` function
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
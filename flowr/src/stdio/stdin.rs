use std::io::{self, Read};

use flowrlib::implementation::DONT_RUN_AGAIN;
use flowrlib::implementation::Implementation;
use flowrlib::implementation::RunAgain;
use serde_json::Value;

pub struct Stdin;

impl Implementation for Stdin {
    fn run(&self, mut _inputs: Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        let mut value = None;

        let mut buffer = String::new();
        if let Ok(size) = io::stdin().read_to_string(&mut buffer) {
            if size > 0 {
                let input = Value::String(buffer.trim().to_string());
                value = Some(input);
            }
        }

        (value, DONT_RUN_AGAIN)
    }
}
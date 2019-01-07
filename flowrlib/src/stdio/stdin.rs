use serde_json::Value as JsonValue;
use super::super::implementation::Implementation;
use super::super::implementation::RunAgain;
use super::super::runnable::Runnable;
use super::super::runlist::RunList;
use std::io::{self, Read};

pub struct Stdin;

impl Implementation for Stdin {
    fn run(&self, runnable: &Runnable, mut _inputs: Vec<Vec<JsonValue>>, run_list: &mut RunList) -> RunAgain {
        let mut buffer = String::new();
        if let Ok(size) = io::stdin().read_to_string(&mut buffer) {
            if size > 0 {
                run_list.send_output(runnable, JsonValue::String(buffer.trim().to_string()));
            }
        }

        false
    }
}
use serde_json::Value as JsonValue;
use std::io::{self};

use super::super::implementation::Implementation;
use super::super::implementation::RunAgain;
use super::super::process::Process;
use super::super::runlist::RunList;

pub struct Readline;

impl Implementation for Readline {
    fn run(&self, process: &Process, _inputs: Vec<Vec<JsonValue>>, run_list: &mut RunList) -> RunAgain {
        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(n) => {
                if n > 0 {
                    run_list.send_output(process, JsonValue::String(input.trim().to_string()));
                    return true;
                }
            }
            Err(_) => {}
        }

        false
    }
}
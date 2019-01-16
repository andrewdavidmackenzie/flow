use std::io::{self, Read};

use flowrlib::implementation::Implementation;
use flowrlib::implementation::RunAgain;
use flowrlib::process::Process;
use flowrlib::runlist::RunList;
use serde_json::Value as JsonValue;

pub struct Stdin;

impl Implementation for Stdin {
    fn run(&self, process: &Process, mut _inputs: Vec<Vec<JsonValue>>, run_list: &mut RunList) -> RunAgain {
        let mut buffer = String::new();
        if let Ok(size) = io::stdin().read_to_string(&mut buffer) {
            if size > 0 {
                run_list.send_output(process, JsonValue::String(buffer.trim().to_string()));
            }
        }

        false
    }
}
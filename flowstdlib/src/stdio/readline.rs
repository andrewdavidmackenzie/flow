use serde_json::Value as JsonValue;
use flowrlib::implementation::Implementation;
use flowrlib::implementation::RunAgain;
use flowrlib::runnable::Runnable;
use flowrlib::runlist::RunList;
use std::io::{self};

pub struct Readline;

impl Implementation for Readline {
    fn run(&self, runnable: &Runnable, _inputs: Vec<Vec<JsonValue>>, run_list: &mut RunList) -> RunAgain {
        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(n) => {
                if n > 0 {
                    run_list.send_output(runnable, JsonValue::String(input.trim().to_string()));
                    return true;
                }
            }
            Err(_) => {}
        }

        false
    }
}
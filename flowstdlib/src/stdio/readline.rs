use serde_json::Value as JsonValue;
use flowrlib::implementation::Implementation;
use flowrlib::runnable::Runnable;
use flowrlib::runlist::RunList;

pub struct Readline;

impl Implementation for Readline {
    fn run(&self, runnable: &Runnable, _inputs: Vec<Vec<JsonValue>>, run_list: &mut RunList) {
        use std::io::{self, BufRead};

        let stdin = io::stdin();
        let mut iterator = stdin.lock().lines();
        if let Some(result) = iterator.next() {
            if let Ok(line) = result {
                run_list.send_output(runnable, JsonValue::String(line));
            }
        }
    }
}
use serde_json::Value as JsonValue;
use flowrlib::implementation::Implementation;
use flowrlib::runnable::Runnable;
use flowrlib::runlist::RunList;
use std::io::{self, Read};

pub struct Stdin;

impl Implementation for Stdin {
    fn run(&self, runnable: &Runnable, mut _inputs: Vec<Vec<JsonValue>>, run_list: &mut RunList) {
        let mut buffer = String::new();
        if let Ok(size) = io::stdin().read_to_string(&mut buffer) {
            if size > 0 {
                run_list.send_output(runnable, JsonValue::String(buffer));
            }
        }
    }
}
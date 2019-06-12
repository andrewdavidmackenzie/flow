use flowrlib::implementation::Implementation;
use flowrlib::implementation::RUN_AGAIN;
use flowrlib::implementation::RunAgain;
use serde_json::Value;

pub struct Readline;

impl Implementation for Readline {
    fn run(&self, _inputs: Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        // TODO convert to web sys
        (Some(json!("readline string")), RUN_AGAIN)
    }
}
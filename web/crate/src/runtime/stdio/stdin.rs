use flowrlib::implementation::Implementation;
use flowrlib::implementation::RUN_AGAIN;
use flowrlib::implementation::RunAgain;
use serde_json::Value;

pub struct Stdin;

impl Implementation for Stdin {
    fn run(&self, mut _inputs: Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        // TODO convert to web sys
        (Some(json!("stdin string")), RUN_AGAIN)
    }
}
use flow_impl::{DONT_RUN_AGAIN, Implementation, RunAgain};
use serde_json::Value;

pub struct Get;

impl Implementation for Get {
    fn run(&self, mut _inputs: Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        // TODO
        let args_text = "ide-native hi".to_string();

        let flow_args: Vec<&str> = args_text.split(' ').collect();

        (Some(json!(flow_args)), DONT_RUN_AGAIN)
    }
}
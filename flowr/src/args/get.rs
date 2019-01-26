use std::env;

use flowrlib::implementation::DONT_RUN_AGAIN;
use flowrlib::implementation::Implementation;
use flowrlib::implementation::RunAgain;
use serde_json::Value as JsonValue;

use super::super::FLOW_ARGS_NAME;

pub struct Get;

impl Implementation for Get {
    fn run(&self, mut _inputs: Vec<Vec<JsonValue>>) -> (Option<JsonValue>, RunAgain) {
        let mut value = None;

        if let Ok(args) = env::var(FLOW_ARGS_NAME) {
            env::remove_var(FLOW_ARGS_NAME); // so another invocation later won't use it by mistake
            let flow_args: Vec<&str> = args.split(' ').collect();
            value = Some(json!(flow_args));
        }

        (value, DONT_RUN_AGAIN)
    }
}
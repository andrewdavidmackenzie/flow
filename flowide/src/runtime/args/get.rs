use flow_impl::{DONT_RUN_AGAIN, Implementation, RunAgain};
use serde_json::Value;

pub struct Get {
    args: Vec<String>
}

impl Get {
    pub fn new(args: &Vec<String>) -> Self {
        Get {
            args: args.clone()
        }
    }
}
impl Implementation for Get {
    fn run(&self, mut _inputs: Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        (Some(json!(self.args)), DONT_RUN_AGAIN)
    }
}
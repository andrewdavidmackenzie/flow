use flow_impl::{Implementation, RUN_AGAIN, RunAgain};
use serde_json::Value;

pub struct Get {
    args: Vec<String>
}

impl Get {
    pub fn new(args: Vec<String>) -> Self {
        Get {
            args
        }
    }
}
impl Implementation for Get {
    fn run(&self, mut _inputs: Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        (Some(json!(self.args)), RUN_AGAIN)
    }
}
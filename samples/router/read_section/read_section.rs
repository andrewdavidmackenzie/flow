use flow_impl::{Implementation, RUN_AGAIN, RunAgain};
use flow_impl_derive::FlowImpl;
use serde_json::json;
use serde_json::Value;

#[derive(FlowImpl, Debug)]
pub struct Router;

impl Implementation for Router {
    fn run(&self, mut inputs: Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        let mut value = None;

        let input_stream = inputs.remove(0);
        let ra = input_stream[0].as_str().unwrap().parse::<u64>();
        let rb = input_stream[1].as_str().unwrap().parse::<u64>();
        let rc = input_stream[2].as_str().unwrap().parse::<u64>();

        match (ra, rb, rc) {
            (Ok(a), Ok(b), Ok(c)) => {
                value = Some(json!(format!("{:?}", [a, b, c]))); // TODO remove format
            }
            _ => {}
        }

        (value, RUN_AGAIN)
    }
}
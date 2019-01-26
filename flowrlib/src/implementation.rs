use serde_json::Value as JsonValue;
use std::panic::RefUnwindSafe;
use std::panic::UnwindSafe;

pub type RunAgain = bool;
pub const RUN_AGAIN: RunAgain = true;
pub const DONT_RUN_AGAIN: RunAgain = false;

pub trait Implementation : RefUnwindSafe + UnwindSafe + Sync {
    // An implementation can be run, with an array of inputs, it can use methods of run_list
    // to send output values and then it eventually returns and indicates with return value whether
    // it should be ran again
    fn run(&self, inputs: Vec<Vec<JsonValue>>) -> (Option<JsonValue>, RunAgain);
}
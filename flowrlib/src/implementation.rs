use serde_json::Value as JsonValue;
use std::panic::{RefUnwindSafe, UnwindSafe};
use std::marker::{Sync, Send};

pub type RunAgain = bool;
pub const RUN_AGAIN: RunAgain = true;
pub const DONT_RUN_AGAIN: RunAgain = false;

pub trait Implementation : RefUnwindSafe + UnwindSafe + Sync + Send {
    // An implementation can be run, with an array of inputs and returns a value (or null) and a
    // bool indicating if it should be ran again
    fn run(&self, inputs: Vec<Vec<JsonValue>>) -> (Option<JsonValue>, RunAgain);
}
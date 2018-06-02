use serde_json::Value as JsonValue;
use std::panic::RefUnwindSafe;
use std::panic::UnwindSafe;
use runnable::Runnable;
use runlist::RunList;

pub type RunAgain = bool;
pub const RUN_AGAIN: RunAgain = true;
pub const DONT_RUN_AGAIN: RunAgain = false;

pub trait Implementation : RefUnwindSafe + UnwindSafe + Sync {
    // An implementation runs, receiving an array of inputs and possibly producing an output
    fn run(&self, runnable: &Runnable, inputs: Vec<Vec<JsonValue>>, run_list: &mut RunList) -> RunAgain;
}
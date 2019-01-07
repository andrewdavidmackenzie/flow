use serde_json::Value as JsonValue;
use implementation::Implementation;
use implementation::RunAgain;
use implementation::RUN_AGAIN;
use runnable::Runnable;
use runlist::RunList;

pub struct Fifo;

impl Implementation for Fifo {
    fn run(&self, runnable: &Runnable, mut inputs: Vec<Vec<JsonValue>>, run_list: &mut RunList) -> RunAgain {
        run_list.send_output(runnable, inputs.remove(0).remove(0));
        RUN_AGAIN
    }
}
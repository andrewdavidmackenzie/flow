use flowrlib::implementation::Implementation;
use flowrlib::implementation::RUN_AGAIN;
use flowrlib::implementation::RunAgain;
use flowrlib::process::Process;
use flowrlib::runlist::RunList;
use serde_json::Value as JsonValue;

pub struct Fifo;

impl Implementation for Fifo {
    fn run(&self, process: &Process, mut inputs: Vec<Vec<JsonValue>>, run_list: &mut RunList) -> RunAgain {
        run_list.send_output(process, inputs.remove(0).remove(0));
        RUN_AGAIN
    }
}
use serde_json::Value as JsonValue;
use flowrlib::implementation::Implementation;
use flowrlib::runnable::Runnable;
use flowrlib::runlist::RunList;

pub struct Fifo;

impl Implementation for Fifo {
    fn run(&self, runnable: &Runnable, mut inputs: Vec<JsonValue>, run_list: &mut RunList) {
        run_list.send_output(runnable, inputs.remove(0));
    }
}
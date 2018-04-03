use serde_json::Value as JsonValue;
use flowrlib::implementation::Implementation;
use flowrlib::runnable::Runnable;
use flowrlib::runlist::RunList;

pub struct Fifo;

impl Implementation for Fifo {
    fn run(&self, runnable: &Runnable, mut inputs: Vec<Vec<JsonValue>>, run_list: &mut RunList) -> bool {
        run_list.send_output(runnable, inputs.remove(0).remove(0));
        true
    }
}
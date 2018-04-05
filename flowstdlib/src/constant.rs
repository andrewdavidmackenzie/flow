use serde_json::Value as JsonValue;
use flowrlib::implementation::Implementation;
use flowrlib::runnable::Runnable;
use flowrlib::runlist::RunList;

pub struct Constant;

impl Implementation for Constant {
    fn run(&self, runnable: &Runnable, inputs: Vec<Vec<JsonValue>>, run_list: &mut RunList) -> bool {
        let only_input = &inputs[0];
        // Never remove the input, reuse it each time
        run_list.send_output(runnable, only_input[0].clone());
        // Indicate we're ready to run again
        true
    }
}
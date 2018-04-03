use serde_json::Value as JsonValue;
use flowrlib::implementation::Implementation;
use flowrlib::runlist::RunList;
use flowrlib::runnable::Runnable;

pub struct ToString;

impl Implementation for ToString {
    fn run(&self, runnable: &Runnable, mut inputs: Vec<Vec<JsonValue>>, run_list: &mut RunList) -> bool {
        let input = inputs.remove(0).remove(0);
        run_list.send_output(runnable, JsonValue::String(input.to_string()));
        true
    }
}
use serde_json::Value as JsonValue;
use serde_json::Value::Number;
use flowrlib::implementation::Implementation;
use flowrlib::runlist::RunList;
use flowrlib::runnable::Runnable;

pub struct ToString;

impl Implementation for ToString {
    fn run(&self, runnable: &Runnable, mut inputs: Vec<JsonValue>, run_list: &mut RunList) {
        let input = inputs.remove(0);
        run_list.send_output(runnable, JsonValue::String(input.to_string()));
    }
}
use serde_json::Value as JsonValue;
use serde_json::Value::String as JsonString;
use flowrlib::implementation::Implementation;
use flowrlib::runnable::Runnable;
use flowrlib::runlist::RunList;

pub struct ReadSection;

impl Implementation for ReadSection {
    fn run(&self, runnable: &Runnable, mut inputs: Vec<JsonValue>, run_list: &mut RunList) {
        let input = inputs.remove(0);
        match input {
            JsonString(ref s) => {
                let output = json!({
                "a" : s,
                "b" : s,
                "c" : s
            });
                run_list.send_output(runnable, output);
            },
            _ => {}
        }
    }
}
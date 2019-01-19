use flowrlib::implementation::Implementation;
use flowrlib::implementation::RunAgain;
use flowrlib::process::Process;
use flowrlib::runlist::RunList;
use serde_json::Value as JsonValue;
use serde_json::Value::String as JsonString;

pub struct Reverse;

impl Implementation for Reverse {
    fn run(&self, process: &Process, mut inputs: Vec<Vec<JsonValue>>, run_list: &mut RunList) -> RunAgain {
        let input = inputs.remove(0).remove(0);
        match input {
            JsonString(ref s) => {
                let output = json!({
                    "reversed" : s.chars().rev().collect::<String>(),
                    "original": s
                });
                run_list.send_output(process, output);
            }
            _ => {}
        }

        true
    }
}
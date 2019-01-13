use flowrlib::implementation::Implementation;
use flowrlib::implementation::RunAgain;
use flowrlib::process::Process;
use flowrlib::runlist::RunList;
use serde_json::Value as JsonValue;

pub struct ToString;

impl Implementation for ToString {
    fn run(&self, process: &Process, mut inputs: Vec<Vec<JsonValue>>, run_list: &mut RunList) -> RunAgain {
        let input = inputs.remove(0).remove(0);
        match input {
            JsonValue::String(_) => {
                run_list.send_output(process, input);
            },
            JsonValue::Bool(boolean) => {
                run_list.send_output(process, JsonValue::String(boolean.to_string()));
            },
            JsonValue::Number(number) => {
                run_list.send_output(process, JsonValue::String(number.to_string()));
            },
            JsonValue::Array(array) => {
                for entry in array {
                    run_list.send_output(process,entry);
                }
            },
            _ => {}
        };

        true
    }
}
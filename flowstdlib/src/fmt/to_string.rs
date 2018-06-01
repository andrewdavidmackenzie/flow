use serde_json::Value as JsonValue;
use flowrlib::implementation::Implementation;
use flowrlib::implementation::RunAgain;
use flowrlib::runlist::RunList;
use flowrlib::runnable::Runnable;

pub struct ToString;

impl Implementation for ToString {
    fn run(&self, runnable: &Runnable, mut inputs: Vec<Vec<JsonValue>>, run_list: &mut RunList) -> RunAgain {
        let input = inputs.remove(0).remove(0);
        match input {
            JsonValue::String(_) => {
                run_list.send_output(runnable, input);
            },
            JsonValue::Bool(boolean) => {
                run_list.send_output(runnable, JsonValue::String(boolean.to_string()));
            },
            JsonValue::Number(number) => {
                run_list.send_output(runnable, JsonValue::String(number.to_string()));
            },
            JsonValue::Array(array) => {
                for entry in array {
                    run_list.send_output(runnable,entry);
                }
            },
            _ => {}
        };

        true
    }
}
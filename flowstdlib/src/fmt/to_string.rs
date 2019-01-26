use flowrlib::implementation::Implementation;
use flowrlib::implementation::RunAgain;
use flowrlib::process::Process;
use flowrlib::runlist::RunList;
use serde_json::Value as JsonValue;

pub struct ToString;

impl Implementation for ToString {
    fn run(&self, process: &Process, mut inputs: Vec<Vec<JsonValue>>, run_list: &mut RunList)
        -> (Option<JsonValue>, RunAgain) {
        let mut value = None;

        let input = inputs.remove(0).remove(0);
        match input {
            JsonValue::String(_) => {
                value = Some(input);
            },
            JsonValue::Bool(boolean) => {
                let val = JsonValue::String(boolean.to_string());
                value = Some(val);
            },
            JsonValue::Number(number) => {
                let val = JsonValue::String(number.to_string());
                value = Some(val);
            },
            _ => {}
        };

        (value, true)
    }
}
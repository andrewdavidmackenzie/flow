use flowrlib::implementation::Implementation;
use flowrlib::implementation::RunAgain;
use flowrlib::process::Process;
use flowrlib::runlist::RunList;
use serde_json;
use serde_json::Value as JsonValue;

pub struct ToNumber;

impl Implementation for ToNumber {
    fn run(&self, process: &Process, mut inputs: Vec<Vec<JsonValue>>, run_list: &mut RunList) -> RunAgain {
        let input = inputs.remove(0).remove(0);

        match input {
            JsonValue::String(string) => {
                if let Ok(number) = string.parse::<i64>() {
                    let number = JsonValue::Number(serde_json::Number::from(number));
                    run_list.send_output(process, number);
                }
            },
            _ => {}
        };

        true
    }
}
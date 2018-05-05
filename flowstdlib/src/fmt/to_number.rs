use serde_json;
use serde_json::Value as JsonValue;
use flowrlib::implementation::Implementation;
use flowrlib::runlist::RunList;
use flowrlib::runnable::Runnable;

pub struct ToNumber;

impl Implementation for ToNumber {
    fn run(&self, runnable: &Runnable, mut inputs: Vec<Vec<JsonValue>>, run_list: &mut RunList) -> bool {
        let input = inputs.remove(0).remove(0);

        match input {
            JsonValue::String(string) => {
                if let Ok(number) = string.parse::<i64>() {
                    let number = JsonValue::Number(serde_json::Number::from(number));
                    run_list.send_output(runnable, number);
                }
            },
            _ => {}
        };

        true
    }
}
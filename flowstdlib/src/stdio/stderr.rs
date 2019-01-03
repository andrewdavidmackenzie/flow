use serde_json::Value as JsonValue;
use flowrlib::implementation::Implementation;
use flowrlib::implementation::RunAgain;
use flowrlib::runnable::Runnable;
use flowrlib::runlist::RunList;

pub struct Stderr;

impl Implementation for Stderr {
    fn run(&self, _runnable: &Runnable, mut inputs: Vec<Vec<JsonValue>>, _run_list: &mut RunList) -> RunAgain {
        let input = inputs.remove(0).remove(0);
        match input {
            JsonValue::String(string) => {
                eprintln!("{}", string);
            },
            JsonValue::Bool(boolean) => {
                eprintln!("{}", JsonValue::String(boolean.to_string()));
            },
            JsonValue::Number(number) => {
                eprintln!("{}", JsonValue::String(number.to_string()));
            },
            JsonValue::Array(array) => {
                for entry in array {
                    eprintln!("{}", entry);
                }
            },
            _ => {}
        };

        true
    }
}
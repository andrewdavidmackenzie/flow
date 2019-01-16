use flowrlib::implementation::Implementation;
use flowrlib::implementation::RunAgain;
use flowrlib::process::Process;
use flowrlib::runlist::RunList;
use serde_json::Value as JsonValue;

pub struct Stderr;

impl Implementation for Stderr {
    fn run(&self, _process: &Process, mut inputs: Vec<Vec<JsonValue>>, _run_list: &mut RunList) -> RunAgain {
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
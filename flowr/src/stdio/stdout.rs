use flowrlib::implementation::Implementation;
use flowrlib::implementation::RunAgain;
use flowrlib::process::Process;
use flowrlib::runlist::RunList;
use serde_json::Value as JsonValue;

pub struct Stdout;

impl Implementation for Stdout {
    fn run(&self, _process: &Process, mut inputs: Vec<Vec<JsonValue>>, _run_list: &mut RunList)
        -> (Option<JsonValue>, RunAgain) {
        let input = inputs.remove(0).remove(0);
        match input {
            JsonValue::String(string) => {
                println!("{}", string);
            },
            JsonValue::Bool(boolean) => {
                println!("{}", JsonValue::String(boolean.to_string()));
            },
            JsonValue::Number(number) => {
                println!("{}", JsonValue::String(number.to_string()));
            },
            JsonValue::Array(array) => {
                for entry in array {
                    println!("{}", entry);
                }
            },
            _ => {}
        };

    (None, true)
    }
}
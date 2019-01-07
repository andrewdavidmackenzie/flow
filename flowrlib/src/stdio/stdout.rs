use serde_json::Value as JsonValue;
use super::super::implementation::Implementation;
use super::super::implementation::RunAgain;
use super::super::runnable::Runnable;
use super::super::runlist::RunList;

pub struct Stdout;

impl Implementation for Stdout {
    fn run(&self, _runnable: &Runnable, mut inputs: Vec<Vec<JsonValue>>, _run_list: &mut RunList) -> RunAgain {
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

        true
    }
}
use flow_impl::implementation::{Implementation, RUN_AGAIN, RunAgain};
use serde_json::Value;

pub struct Stderr;

impl Implementation for Stderr {
    fn run(&self, mut inputs: Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        let input = inputs.remove(0).remove(0);

        let window = web_sys::window().expect("no global `window` exists");
        let document = window.document().expect("should have a document on window");
        let stderr = document.get_element_by_id("stderr").expect("could not find 'stderr' element");

        match input {
            Value::String(string) => {
                let text = document.create_text_node(&string);
                stderr.append_child(&text).unwrap();
            },
            Value::Bool(boolean) => {
                let text = document.create_text_node(&boolean.to_string());
                stderr.append_child(&text).unwrap();
            },
            Value::Number(number) => {
                let text = document.create_text_node(&number.to_string());
                stderr.append_child(&text).unwrap();
            },
            Value::Array(array) => {
                for entry in array {
                    let text = document.create_text_node(&entry.to_string());
                    stderr.append_child(&text).unwrap();
                }
            },
            _ => {}
        };

        (None, RUN_AGAIN)
    }
}
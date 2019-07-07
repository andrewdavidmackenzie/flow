use flowrlib::implementation::Implementation;
use flowrlib::implementation::RUN_AGAIN;
use flowrlib::implementation::RunAgain;
use serde_json::Value;

pub struct Stdout;

impl Implementation for Stdout {
    fn run(&self, mut inputs: Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        let input = inputs.remove(0).remove(0);

        let window = web_sys::window().expect("no global `window` exists");
        let document = window.document().expect("should have a document on window");
        let stdout = document.get_element_by_id("stdout").expect("could not find 'stdout' element");

        match input {
            Value::String(string) => {
                let text = document.create_text_node(&string);
                stdout.append_child(&text).unwrap();
            },
            Value::Bool(boolean) => {
                let text = document.create_text_node(&boolean.to_string());
                stdout.append_child(&text).unwrap();
            },
            Value::Number(number) => {
                let text = document.create_text_node(&number.to_string());
                stdout.append_child(&text).unwrap();
            },
            Value::Array(array) => {
                for entry in array {
                    let text = document.create_text_node(&entry.to_string());
                    stdout.append_child(&text).unwrap();
                }
            },
            _ => {}
        };

    (None, RUN_AGAIN)
    }
}
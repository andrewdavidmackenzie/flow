use flow_impl::{Implementation, RUN_AGAIN, RunAgain};
use gtk::TextBuffer;
use serde_json::Value;

pub struct Stderr  {
    stderr: TextBuffer
}

impl Stderr {
    pub fn new() -> Self {
        Stderr {
            stderr: TextBuffer::new()
        }
    }

    pub fn get_text_buffer(&self) -> &TextBuffer {
        &self.stderr
    }
}

unsafe impl Send for Stderr {}
unsafe impl Sync for Stderr {}

impl Implementation for Stderr {
    fn run(&self, mut inputs: Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        let input = inputs.remove(0).remove(0);

        // TODO

        match input {
            Value::String(string) => {
                eprintln!("{}", string);
            },
            Value::Bool(boolean) => {
                eprintln!("{}", boolean);
            },
            Value::Number(number) => {
                eprintln!("{}", number);
            },
            Value::Array(array) => {
                for entry in array {
                    eprintln!("{}", entry);
                }
            },
            _ => {}
        };

        (None, RUN_AGAIN)
    }
}
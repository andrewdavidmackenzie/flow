use flow_impl::{Implementation, RUN_AGAIN, RunAgain};
use gtk::TextBuffer;
use serde_json::Value;

pub struct Stdout {
    stdout: TextBuffer
}

impl Stdout {
    pub fn new() -> Self {
        Stdout {
            stdout: TextBuffer::new()
        }
    }

    pub fn get_text_buffer(&self) -> &TextBuffer {
        &self.stdout
    }
}

unsafe impl Send for Stdout {}
unsafe impl Sync for Stdout {}

impl Implementation for Stdout {
    fn run(&self, mut inputs: Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        let input = inputs.remove(0).remove(0);

        // TODO

        match input {
            Value::String(string) => {
                println!("{}", string);
            },
            Value::Bool(boolean) => {
                println!("{}", boolean);
            },
            Value::Number(number) => {
                println!("{}", number);
            },
            Value::Array(array) => {
                for entry in array {
                    println!("{}", entry);
                }
            },
            _ => {}
        };

    (None, RUN_AGAIN)
    }
}
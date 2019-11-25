use flow_impl::{Implementation, RUN_AGAIN, RunAgain};
use gtk::TextBuffer;
use serde_json::Value;
use std::sync::{Arc, Mutex};

pub struct Get {
    args: Mutex<Arc<TextBuffer>>
}

impl Get {
    pub fn new() -> Self {
        Get {
            args: Mutex::new(Arc::new(TextBuffer::new(gtk::NONE_TEXT_TAG_TABLE)))
        }
    }

    pub fn get_text_buffer(&self) -> &TextBuffer {
        &self.args
    }
}

unsafe impl Send for Get {}
unsafe impl Sync for Get {}

impl Implementation for Get {
    fn run(&self, mut _inputs: Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        let (start, end) = self.args.get_bounds();
        let args_string = self.args.get_text(&start, &end, false).unwrap().to_string();
        let arg_values: Vec<String> = args_string.split(' ').map(|s| s.to_string()).collect();

        (Some(json!(self.args)), RUN_AGAIN)
    }
}
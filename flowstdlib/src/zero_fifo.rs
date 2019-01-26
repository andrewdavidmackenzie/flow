use flowrlib::implementation::Implementation;
use flowrlib::implementation::RUN_AGAIN;
use flowrlib::implementation::RunAgain;
use serde_json::Value as JsonValue;

pub struct Fifo;

impl Implementation for Fifo {
    fn run(&self, mut inputs: Vec<Vec<JsonValue>>) -> (Option<JsonValue>, RunAgain) {
        let value = Some(inputs.remove(0).remove(0));
        (value, RUN_AGAIN)
    }
}
use flowrlib::implementation::Implementation;
use flowrlib::implementation::RUN_AGAIN;
use flowrlib::implementation::RunAgain;
use serde_json::Value;

pub struct ComposeArray;

impl Implementation for ComposeArray {
    fn run(&self, mut inputs: Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        let mut input_stream = inputs.remove(0);
        let a = input_stream.remove(0);
        let b = input_stream.remove(0);
        let c = input_stream.remove(0);
        let d = input_stream.remove(0);

        (Some(json!([a, b, c, d])), RUN_AGAIN)
    }
}
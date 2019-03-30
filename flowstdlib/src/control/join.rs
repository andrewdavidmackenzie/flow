use flowrlib::implementation::Implementation;
use flowrlib::implementation::RunAgain;
use flowrlib::implementation::RUN_AGAIN;
use serde_json::Value;

pub struct Join;

/*
    A function that outputs the "data" input once the second input "control" is available and
    the function can run
*/
impl Implementation for Join {
    fn run(&self, mut inputs: Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        let data = Some(inputs[0].remove(0));
        let control = inputs[1].remove(0);

        (data, RUN_AGAIN)
    }
}
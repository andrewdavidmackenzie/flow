use flowrlib::implementation::Implementation;
use flowrlib::implementation::RunAgain;
use flowrlib::implementation::RUN_AGAIN;
use serde_json::Value;

pub struct Tap;

/*
    A control switch function that outputs the "data" input IF the "control" input is true,
    otherwise it does not produce any output
*/
impl Implementation for Tap {
    fn run(&self, mut inputs: Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        let mut value = None;
        let data = inputs[0].remove(0);
        let control = inputs[1].remove(0).as_bool().unwrap();
        if control {
            value = Some(data);
        }

        (value, RUN_AGAIN)
    }
}
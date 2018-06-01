use serde_json::Value as JsonValue;
use flowrlib::implementation::Implementation;
use flowrlib::implementation::RunAgain;
use flowrlib::runnable::Runnable;
use flowrlib::runlist::RunList;

pub struct Stderr;

impl Implementation for Stderr {
    fn run(&self, _runnable: &Runnable, mut inputs: Vec<Vec<JsonValue>>, _run_list: &mut RunList) -> RunAgain {
        eprintln!("{}", inputs.remove(0).get(0).unwrap().as_str().unwrap());
        true
    }
}
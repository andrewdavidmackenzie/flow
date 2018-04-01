use serde_json::Value as JsonValue;
use flowrlib::implementation::Implementation;
use flowrlib::runnable::Runnable;
use flowrlib::runlist::RunList;

pub struct Stderr;

impl Implementation for Stderr {
    fn run(&self, _runnable: &Runnable, mut inputs: Vec<JsonValue>, _run_list: &mut RunList) {
        eprintln!("{}", inputs.remove(0).as_str().unwrap());
    }
}
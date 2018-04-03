use serde_json::Value as JsonValue;
use flowrlib::implementation::Implementation;
use flowrlib::runnable::Runnable;
use flowrlib::runlist::RunList;

pub struct Stdout;

impl Implementation for Stdout {
    fn run(&self, _runnable: &Runnable, mut inputs: Vec<Vec<JsonValue>>, _run_list: &mut RunList) -> bool {
        println!("{}", inputs.remove(0).get(0).unwrap().as_str().unwrap());
        true
    }
}
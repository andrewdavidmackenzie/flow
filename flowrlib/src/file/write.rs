use serde_json::Value as JsonValue;
use super::super::implementation::Implementation;
use super::super::implementation::RunAgain;
use super::super::runnable::Runnable;
use super::super::runlist::RunList;

pub struct Write;

impl Implementation for Write {
    fn run(&self, _runnable: &Runnable, mut _inputs: Vec<Vec<JsonValue>>, _run_list: &mut RunList) -> RunAgain {
        // TODO get filename and bytes from inputs and write them to a file, creating file and intermediate
        // directories if needed

        false
    }
}
use std::fs::File;
use std::io::Write;
use serde_json::Value as JsonValue;
use super::super::implementation::Implementation;
use super::super::implementation::RunAgain;
use super::super::runnable::Runnable;
use super::super::runlist::RunList;

pub struct FileWrite;

impl Implementation for FileWrite {
    fn run(&self, _runnable: &Runnable, mut inputs: Vec<Vec<JsonValue>>, _run_list: &mut RunList) -> RunAgain {
        let filename = inputs.remove(0).remove(0);
        let bytes = inputs.remove(0).remove(0);
        let mut file = File::create(filename.as_str().unwrap()).unwrap();

        file.write(bytes.as_str().unwrap().as_bytes()).unwrap();

        false
    }
}
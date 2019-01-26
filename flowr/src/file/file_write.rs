use std::fs::File;
use std::io::Write;

use flowrlib::implementation::Implementation;
use flowrlib::implementation::RunAgain;
use flowrlib::process::Process;
use flowrlib::runlist::RunList;
use serde_json::Value as JsonValue;

pub struct FileWrite;

impl Implementation for FileWrite {
    fn run(&self, _process: &Process, mut inputs: Vec<Vec<JsonValue>>, _run_list: &mut RunList)
        -> (Option<JsonValue>, RunAgain) {
        let filename = inputs.remove(0).remove(0);
        let bytes = inputs.remove(0).remove(0);
        let mut file = File::create(filename.as_str().unwrap()).unwrap();

        file.write(bytes.as_str().unwrap().as_bytes()).unwrap();

        (None, false)
    }
}
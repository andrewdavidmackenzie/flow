use std::fs::File;
use std::io::Write;

use flowrlib::implementation::DONT_RUN_AGAIN;
use flowrlib::implementation::Implementation;
use flowrlib::implementation::RunAgain;
use serde_json::Value;

pub struct FileWrite;

impl Implementation for FileWrite {
    fn run(&self, mut inputs: Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        let filename = inputs.remove(0).remove(0);
        let bytes = inputs.remove(0).remove(0);
        let mut file = File::create(filename.as_str().unwrap()).unwrap();

        file.write(bytes.as_str().unwrap().as_bytes()).unwrap();

        (None, DONT_RUN_AGAIN)
    }
}
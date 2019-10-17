use flow_impl::{DONT_RUN_AGAIN, Implementation, RunAgain};
use serde_json::Value;

pub struct FileWrite;

impl Implementation for FileWrite {
    fn run(&self, mut inputs: Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        let _filename = inputs.remove(0).remove(0);
        let _bytes = inputs.remove(0).remove(0);

        // TODO convert to flowide-sys
//        let mut _file = File::create(filename.as_str().unwrap()).unwrap();
//        file.write(bytes.as_str().unwrap().as_bytes()).unwrap();

        (None, DONT_RUN_AGAIN)
    }
}
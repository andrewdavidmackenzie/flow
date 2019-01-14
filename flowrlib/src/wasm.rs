use implementation::Implementation;
use implementation::RUN_AGAIN;
use implementation::RunAgain;
use process::Process;
use runlist::RunList;
use serde_json::Value as JsonValue;

pub struct Wasm;

// TODO an implementation wrapper for a file loaded from WASM bytecode, it will use a wasm object
// created by the loader at library load time to located what to execute.
impl Implementation for Wasm {
    fn run(&self, process: &Process, mut inputs: Vec<Vec<JsonValue>>, run_list: &mut RunList) -> RunAgain {
        run_list.send_output(process, inputs.remove(0).remove(0));
        RUN_AGAIN
    }
}
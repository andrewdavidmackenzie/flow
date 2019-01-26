use std::sync::{Arc, Mutex};

use implementation::Implementation;
use implementation::RunAgain;
use process::Process;
use runlist::RunList;
use wasmi::ModuleRef;
use serde_json::Value as JsonValue;

pub struct WasmExecutor {
    pub module: Arc<Mutex<Arc<ModuleRef>>>
}

impl Implementation for WasmExecutor {
    fn run(&self, _process: &Process, _inputs: Vec<Vec<JsonValue>>, _run_list: &mut RunList)
        -> (Option<JsonValue>, RunAgain) {
        println!("Wasm implementation wrapper called");

        (None, true)
    }
}
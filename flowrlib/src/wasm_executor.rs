use std::sync::{Arc, Mutex};

use implementation::Implementation;
use implementation::RunAgain;
use wasmi::ModuleRef;
use serde_json::Value as JsonValue;

pub struct WasmExecutor {
    pub module: Arc<Mutex<Arc<ModuleRef>>>
}

impl Implementation for WasmExecutor {
    fn run(&self, _inputs: Vec<Vec<JsonValue>>) -> (Option<JsonValue>, RunAgain) {
        println!("Wasm implementation wrapper called");
        (None, true)
    }
}
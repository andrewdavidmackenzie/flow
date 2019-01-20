use std::sync::{Arc, Mutex};

use implementation::Implementation;
use implementation::RunAgain;
use process::Process;
use runlist::RunList;
use provider::Provider;
use wasmi::{Module, ImportsBuilder, ModuleInstance, ModuleRef};
use serde_json::Value as JsonValue;
use url::Url;

pub struct WasmExecutor {
//    module: Arc<Mutex<ModuleRef>>
}

impl Implementation for WasmExecutor {
    fn run(&self, _process: &Process, _inputs: Vec<Vec<JsonValue>>, _run_list: &mut RunList) -> RunAgain {
        println!("Wasm implementation wrapper called");
        true
    }
}

impl WasmExecutor {
    pub fn wrap_wasm<'a>(provider: &Provider, source_url: &Url) -> Result<&'a WasmExecutor, String> {
        let (resolved_url, _) = provider.resolve(source_url)?;
        let content = provider.get(&resolved_url)?;

        // TODO optimization, make a module only once from each source wasm file then make
        // multiple instances of it or references to it??
        let module = Module::from_buffer(content)
            .map_err(|e| e.to_string())?;

        let module = ModuleInstance::new(&module,
                                         &ImportsBuilder::default())
            .map_err(|e| e.to_string())?
            .assert_no_start();

//        Ok(&WasmExecutor { module: Arc::new(Mutex::new(module)) })
        Ok(&WasmExecutor {})
    }
}
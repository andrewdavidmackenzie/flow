use std::sync::{Arc, Mutex};

use implementation::Implementation;
use implementation::RunAgain;
use serde_json::Value as JsonValue;
use provider::Provider;
use std::rc::Rc;

#[cfg(not(target_arg = "wasm32"))]
use wasmi::{Module, ModuleRef, ModuleInstance, ImportsBuilder};

#[cfg(not(target_arg = "wasm32"))]
pub struct WasmExecutor {
    pub module: Arc<Mutex<Arc<ModuleRef>>>
}
#[cfg(target_arg = "wasm32")]
pub struct WasmExecutor {
}

impl Implementation for WasmExecutor {
    fn run(&self, _inputs: Vec<Vec<JsonValue>>) -> (Option<JsonValue>, RunAgain) {
        println!("Wasm implementation wrapper called");
        (None, true)
    }
}

/*
    load a Wasm module from the specified Url.
*/
pub fn load(provider: &Provider, source_url: &str)
                 -> Result<Rc<WasmExecutor>, String> {
    let (resolved_url, _) = provider.resolve(&source_url)?;
    let content = provider.get(&resolved_url)?;

    let module = Module::from_buffer(content)
        .map_err(|e| e.to_string())?;

    let module_ref = Arc::new(ModuleInstance::new(&module,
                                                  &ImportsBuilder::default())
        .map_err(|e| e.to_string())?
        .assert_no_start());

    let executor = WasmExecutor { module: Arc::new(Mutex::new(module_ref.clone())) };

    Ok(Rc::new(executor))
}
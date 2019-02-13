use std::sync::{Arc, Mutex};

use implementation::Implementation;
use implementation::RunAgain;
use serde_json::Value as JsonValue;
use provider::Provider;

#[cfg(not(target_arg = "wasm32"))]
use wasmi::{Module, ModuleRef, ModuleInstance, ImportsBuilder};
//use wasmi::{Module, ModuleRef, ModuleInstance, ImportsBuilder, RuntimeValue, NopExternals};

const DEFAULT_WASM_FILENAME: &str = "module.wasm";

#[cfg(not(target_arg = "wasm32"))]
pub struct WasmExecutor {
    pub module: Arc<Mutex<ModuleRef>>,
    _function_name: String,
}

#[cfg(target_arg = "wasm32")]
pub struct WasmExecutor {}

/*
    A wasm module is invoked thus:
        pub fn invoke_export<E: Externals>(&self, func_name: &str, args: &[RuntimeValue],
            externals: &mut E) -> Result<Option<RuntimeValue>, Error>
*/
impl Implementation for WasmExecutor {
    fn run(&self, _inputs: Vec<Vec<JsonValue>>) -> (Option<JsonValue>, RunAgain) {
        let _module: &mut ModuleRef = &mut *self.module.lock().unwrap();

        println!("Wasm implementation wrapper called");

        /*
        let res = module.invoke_export(self.function_name, &[RuntimeValue::from(inputs)],
                                  &mut NopExternals).unwrap().unwrap();
        res
        */
        (None, true)
    }
}

/*
    load a Wasm module from the specified Url.
*/
pub fn load(provider: &Provider, name: &str, source_url: &str)
            -> Result<Arc<WasmExecutor>, String> {
    let (resolved_url, _) = provider.resolve(&source_url, DEFAULT_WASM_FILENAME)?;
    let content = provider.get(&resolved_url)?;

    let module = Module::from_buffer(content)
        .map_err(|e| e.to_string())?;

    let module_ref = ModuleInstance::new(&module,
                                         &ImportsBuilder::default())
        .map_err(|e| e.to_string())?
        .assert_no_start();

    let executor = WasmExecutor {
        module: Arc::new(Mutex::new(module_ref.clone())),
        _function_name: name.to_string()
    };

    Ok(Arc::new(executor))
}
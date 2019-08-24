use std::sync::Arc;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Mutex;

use serde_json::Value;
#[cfg(not(target_arch = "wasm32"))]
use wasmi::{ImportsBuilder, Module, ModuleInstance, ModuleRef};

use crate::errors::*;
use crate::implementation::Implementation;
use crate::implementation::RunAgain;
use crate::provider::Provider;

#[cfg(not(target_arch = "wasm32"))]
const DEFAULT_WASM_FILENAME: &str = "module.wasm";

#[cfg(not(target_arch = "wasm32"))]
pub struct WasmExecutor {
    pub module: Arc<Mutex<ModuleRef>>,
    function_name: String,
}

#[cfg(target_arch = "wasm32")]
pub struct WasmExecutor;

/*

*/
impl Implementation for WasmExecutor {
    fn run(&self, _inputs: Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        #[cfg(not(target_arch = "wasm32"))]
        println!("Wasm implementation wrapper for function '{}' called", self.function_name);

        // TODO setup module memory
        // TODO call the wasm implementation function (by name?) and get the result
        // TODO read the module memory and reconstruct the return tuple

        /*
          A wasm module is invoked thus:
            pub fn invoke_export<E: Externals>(&self, func_name: &str, args: &[RuntimeValue],
                                    externals: &mut E) -> Result<Option<RuntimeValue>, Error>

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
#[cfg(target_arch = "wasm32")]
pub fn load(_provider: &dyn Provider, _function_name: &str, _source_url: &str) -> Result<Arc<WasmExecutor>> {
    let executor = WasmExecutor {};
    Ok(Arc::new(executor))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load(provider: &dyn Provider, function_name: &str, source_url: &str) -> Result<Arc<WasmExecutor>> {
    let (resolved_url, _) = provider.resolve(&source_url, DEFAULT_WASM_FILENAME)?;
    let content = provider.get(&resolved_url)?;

    let module = Module::from_buffer(content)
        .chain_err(|| format!("Could not create Wasm Module from content in '{}'", resolved_url))?;

    let module_ref = ModuleInstance::new(&module, &ImportsBuilder::default())
        .chain_err(|| "Could not create new ModuleInstance when loading WASM content")?
        .assert_no_start();

    let executor = WasmExecutor {
        module: Arc::new(Mutex::new(module_ref.clone())),
        function_name: function_name.to_string(),
    };

    Ok(Arc::new(executor))
}
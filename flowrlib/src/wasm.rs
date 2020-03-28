#[cfg(not(target_arch = "wasm32"))]
use std::cmp::max;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Arc;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Mutex;

use flow_impl::{Implementation, RunAgain};
use log::{error, info, trace};
use serde_json::Value;
#[cfg(not(target_arch = "wasm32"))]
use wasmi::{ExternVal, ImportsBuilder, MemoryRef, Module, ModuleInstance, ModuleRef,
            NopExternals, RuntimeValue, Signature, ValueType};

use crate::errors::*;
use crate::provider::Provider;

#[cfg(not(target_arch = "wasm32"))]
const DEFAULT_WASM_FILENAME: &str = "module";

#[cfg(not(target_arch = "wasm32"))]
const MAX_RESULT_SIZE: i32 = 1024;

#[cfg(target_arch = "wasm32")]
#[derive(Debug)]
pub struct WasmExecutor;

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug)]
pub struct WasmExecutor {
    module: Arc<Mutex<ModuleRef>>,
    memory: Arc<Mutex<MemoryRef>>,
}

#[cfg(not(target_arch = "wasm32"))]
impl WasmExecutor {
    pub fn new(module_ref: ModuleRef, memory: MemoryRef) -> Self {
        WasmExecutor {
            module: Arc::new(Mutex::new(module_ref)),
            memory: Arc::new(Mutex::new(memory)),
        }
    }
}

unsafe impl Send for WasmExecutor {}
unsafe impl Sync for WasmExecutor {}

/*
    Allocate memory for array of bytes inside the wasm module and copy the array of bytes into it
*/
#[cfg(not(target_arch = "wasm32"))]
fn send_byte_array(instance: &ModuleRef, memory: &MemoryRef, bytes: &[u8]) -> u32 {
    let alloc_size = max(bytes.len() as i32, MAX_RESULT_SIZE);
    let result = instance
        .invoke_export("alloc", &[RuntimeValue::I32(alloc_size)],
                       &mut NopExternals);

    match result.unwrap().unwrap() {
        RuntimeValue::I32(pointer) => {
            memory.set(pointer as u32, bytes).unwrap();
            pointer as u32
        }
        _ => 0 as u32
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Implementation for WasmExecutor {
    fn run(&self, inputs: &Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        #[cfg(not(target_arch = "wasm32"))]
            let module_ref = self.module.lock().unwrap();
        let memory_ref = self.memory.lock().unwrap();

        // setup module memory with the serde serialization of `inputs: Vec<Vec<Value>>`
        let input_data = serde_json::to_vec(&inputs).unwrap();

        // Allocate a string for the input data inside wasm module
        let input_data_wasm_ptr = send_byte_array(&module_ref, &memory_ref, &input_data);

        let result = module_ref.invoke_export("run_wasm",
                                              &[RuntimeValue::I32(input_data_wasm_ptr as i32),
                                                  RuntimeValue::I32(input_data.len() as i32), ], &mut NopExternals);

        match result {
            Ok(value) => {
                match value.unwrap() {
                    RuntimeValue::I32(result_length) => {
                        trace!("Return length from wasm function of {}", result_length);
                        if result_length > MAX_RESULT_SIZE {
                            error!("Return length from wasm function of {} exceed maximum allowed", result_length);
                            (None, true)
                        } else {
                            let result_data = memory_ref.get(input_data_wasm_ptr, result_length as usize).unwrap();
                            let (result, run_again) = serde_json::from_slice(result_data.as_slice()).unwrap();
                            (result, run_again)
                        }
                    }
                    _ => {
                        error!("Unexpected return value from wasm function on invoke_export()");
                        (None, true)
                    }
                }
            }
            Err(err) => {
                error!("Error returned by Wasm invoke_export(): {:?}", err);
                (None, true)
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
impl Implementation for WasmExecutor {
    fn run(&self, _inputs: Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        (None, false)
    }
}

/*
    load a Wasm module from the specified Url.
*/
#[cfg(not(target_arch = "wasm32"))]
pub fn load(provider: &dyn Provider, source_url: &str) -> Result<WasmExecutor> {
    let (resolved_url, _) = provider.resolve_url(&source_url, DEFAULT_WASM_FILENAME, &["wasm"])?;
    let content = provider.get_contents(&resolved_url)?;
    let module = Module::from_buffer(content)
        .chain_err(|| format!("Could not create Wasm Module from content in '{}'", resolved_url))?;

    let module_ref = ModuleInstance::new(&module, &ImportsBuilder::default())
        .chain_err(|| "Could not create new ModuleInstance when loading WASM content")?
        .assert_no_start();

    let memory = module_ref.export_by_name("memory")
        .chain_err(|| "`memory` export not found")?
        .as_memory()
        .chain_err(|| "export name `memory` is not of memory type")?
        .to_owned();

    check_required_functions(&module_ref, &resolved_url)?;

    info!("Loaded wasm module from: '{}'", source_url);

    Ok(WasmExecutor::new(module_ref, memory))
}

#[cfg(not(target_arch = "wasm32"))]
fn check_required_functions(module_ref: &ModuleRef, filename: &str) -> Result<()> {
    let required_wasm_functions = vec!(
        ("alloc", Signature::new(&[ValueType::I32][..], Some(ValueType::I32))),
        ("run_wasm", Signature::new(&[ValueType::I32, ValueType::I32][..], Some(ValueType::I32))),
    );

    for (function_name, signature) in required_wasm_functions {
        match module_ref.export_by_name(function_name).ok_or(format!("No function named '{}' found in wasm file '{}'",
                                                                     function_name, filename))? {
            ExternVal::Func(function_ref) => {
                let sig = function_ref.signature();
                if *sig != signature {
                    bail!("Expected function signature '{:?}' and found signature '{:?}'",
                            signature, sig);
                }
            }
            _ => bail!("Exported value was not a function")
        }
    }

    Ok(())
}
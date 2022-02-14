use flowcore::{Implementation, RunAgain};
use flowcore::lib_provider::Provider;
use flowcore::errors::*;
use log::{info, trace};
use serde_json::Value;
use std::cmp::max;
use std::sync::Arc;
use std::sync::Mutex;
use url::Url;
use wasmi::{
    ExternVal, ImportsBuilder, MemoryRef, Module, ModuleInstance, ModuleRef, NopExternals,
    RuntimeValue, Signature, ValueType,
};

const DEFAULT_WASM_FILENAME: &str = "module";

const MAX_RESULT_SIZE: i32 = 1024;

#[derive(Debug)]
pub struct WasmExecutor {
    module: Arc<Mutex<ModuleRef>>,
    memory: Arc<Mutex<MemoryRef>>,
    source_url: Url,
}

impl WasmExecutor {
    fn new(module_ref: ModuleRef, memory: MemoryRef, source_url: &Url) -> Self {
        WasmExecutor {
            module: Arc::new(Mutex::new(module_ref)),
            memory: Arc::new(Mutex::new(memory)),
            source_url: source_url.clone(),
        }
    }
}

unsafe impl Send for WasmExecutor {}

unsafe impl Sync for WasmExecutor {}

/*
    Allocate memory for array of bytes inside the wasm module and copy the array of bytes into it
*/
fn send_byte_array(instance: &ModuleRef, memory: &MemoryRef, bytes: &[u8]) -> u32 {
    let alloc_size = max(bytes.len() as i32, MAX_RESULT_SIZE);
    let result =
        instance.invoke_export("alloc", &[RuntimeValue::I32(alloc_size)], &mut NopExternals);

    match result {
        Ok(Some(RuntimeValue::I32(pointer))) => match memory.set(pointer as u32, bytes) {
            Ok(_) => pointer as u32,
            _ => 0_u32,
        },
        _ => 0_u32,
    }
}

impl Implementation for WasmExecutor {
    fn run(&self, inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
        if let (Ok(module_ref), Ok(memory_ref)) = (self.module.lock(), self.memory.lock()) {
            // setup module memory with the serde serialization of `inputs: Vec<Vec<Value>>`
            if let Ok(input_data) = serde_json::to_vec(&inputs) {
                // Allocate a string for the input data inside wasm module
                let input_data_wasm_ptr = send_byte_array(&module_ref, &memory_ref, &input_data);

                let result = module_ref.invoke_export(
                    "run_wasm",
                    &[
                        RuntimeValue::I32(input_data_wasm_ptr as i32),
                        RuntimeValue::I32(input_data.len() as i32),
                    ],
                    &mut NopExternals,
                );

                return match result {
                    Ok(Some(value)) => match value {
                        RuntimeValue::I32(result_length) => {
                            trace!("Return length from wasm function of {}", result_length);
                            if result_length > MAX_RESULT_SIZE {
                                bail!(
                                    "Return length from wasm function of {} exceed maximum allowed",
                                    result_length
                                );
                            }

                            if let Ok(result_data) =
                                memory_ref.get(input_data_wasm_ptr, result_length as usize)
                            {
                                if let Ok((result, run_again)) =
                                    serde_json::from_slice(result_data.as_slice())
                                {
                                    Ok((result, run_again))
                                } else {
                                    bail!("Could not get json result");
                                }
                            } else {
                                bail!("could not get() memory_reference");
                            }
                        }
                        _ => {
                            bail!("Unexpected return value from wasm function on invoke_export()");
                        }
                    },
                    Ok(None) => {
                        bail!(format!("None value returned by Wasm invoke_export(): {:?}\nInputs:\n{:?}",
                            self.source_url, inputs));
                    }
                    Err(err) => {
                        bail!("Error returned by Wasm invoke_export() on '{}': {:?}\nInputs:\n{:?}",
                            self.source_url, err, inputs);
                    }
                };
            }
        }

        Ok((None, true))
    }
}

/// load a Wasm module from the specified Url.
pub fn load(provider: &dyn Provider, source_url: &Url) -> Result<WasmExecutor> {
    let (resolved_url, _) = provider
        .resolve_url(source_url, DEFAULT_WASM_FILENAME, &["wasm"])
        .chain_err(|| "Could not resolve url for manifest while attempting to load manifest")?;
    let content = provider.get_contents(&resolved_url).chain_err(|| {
        format!(
            "Could not fetch content from url '{}' for loading wasm",
            resolved_url
        )
    })?;
    let module = Module::from_buffer(content).chain_err(|| {
        format!(
            "Could not create Wasm Module from content in '{}'",
            resolved_url
        )
    })?;

    let module_ref = ModuleInstance::new(&module, &ImportsBuilder::default())
        .chain_err(|| "Could not create new ModuleInstance when loading WASM content")?
        .assert_no_start();

    let memory = module_ref
        .export_by_name("memory")
        .chain_err(|| "`memory` export not found")?
        .as_memory()
        .chain_err(|| "export name `memory` is not of memory type")?
        .to_owned();

    check_required_functions(&module_ref, &resolved_url)?;

    info!("Loaded wasm module from: '{}'", source_url);

    Ok(WasmExecutor::new(module_ref, memory, source_url))
}

fn check_required_functions(module_ref: &ModuleRef, filename: &Url) -> Result<()> {
    let required_wasm_functions = vec![
        (
            "alloc",
            Signature::new(&[ValueType::I32][..], Some(ValueType::I32)),
        ),
        (
            "run_wasm",
            Signature::new(&[ValueType::I32, ValueType::I32][..], Some(ValueType::I32)),
        ),
    ];

    for (function_name, signature) in required_wasm_functions {
        match module_ref.export_by_name(function_name).ok_or(format!(
            "No function named '{}' found in wasm file '{}'",
            function_name, filename
        ))? {
            ExternVal::Func(function_ref) => {
                let sig = function_ref.signature();
                if *sig != signature {
                    bail!(
                        "Expected function signature '{:?}' and found signature '{:?}'",
                        signature,
                        sig
                    );
                }
            }
            _ => bail!("Exported value was not a function"),
        }
    }

    Ok(())
}

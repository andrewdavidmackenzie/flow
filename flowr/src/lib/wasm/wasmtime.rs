use flowcore::lib_provider::Provider;
use flowcore::{Implementation, RunAgain};
use log::{error, info, trace};
use serde_json::Value;
use std::cmp::max;
use std::sync::Arc;
use std::sync::Mutex;
use url::Url;
use wasmtime::{Engine, Func};

use crate::errors::*;

const DEFAULT_WASM_FILENAME: &str = "module";

const MAX_RESULT_SIZE: i32 = 1024;

#[derive(Debug)]
pub struct WasmExecutor {
    store: Store,
    instance: Instance,
    implementation: Func,
    alloc: Func,
    source_url: Url,
}

impl WasmExecutor {
    pub fn new(
        store: Store,
        instance: Instance,
        implementation: Func,
        alloc: Func,
        source_url: &Url,
    ) -> Self {
        WasmExecutor {
            store,
            instance,
            implementation,
            alloc,
            source_url: source_url.clone(),
        }
    }

    // Allocate memory for array of bytes inside the wasm module and copy the array of bytes into it
    fn send_byte_array(&self, bytes: &[u8]) -> u32 {
        let alloc_size = max(bytes.len() as i32, MAX_RESULT_SIZE);
        let result = self.alloc.call(
            &self.store,
            &[RuntimeValue::I32(alloc_size)],
            &mut NopExternals,
        );

        match result {
            Ok(Some(RuntimeValue::I32(pointer))) => match memory.set(pointer as u32, bytes) {
                Ok(_) => pointer as u32,
                _ => 0_u32,
            },
            _ => 0_u32,
        }
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

    let engine = Engine::default();

    let module = Module::new(&engine, content).chain_err(|| {
        format!(
            "Could not create Wasm Module from content in '{}'",
            resolved_url
        )
    })?;

    let mut store = Store::new(&engine, ());

    let instance = Instance::new(&store, &module, &[])?;

    let implementation = instance.get_func(&mut store, "run_wasm")?;
    let alloc = instance.get_func(&mut store, "alloc")?;

    info!("Loaded wasm module from: '{}'", source_url);

    Ok(WasmExecutor::new(
        store,
        instance,
        implementation,
        alloc,
        source_url,
    ))
}

unsafe impl Send for WasmExecutor {}

unsafe impl Sync for WasmExecutor {}

impl Implementation for WasmExecutor {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain) {
        // setup module memory with the serde serialization of `inputs: Vec<Vec<Value>>`
        if let Ok(input_data) = serde_json::to_vec(&inputs) {
            // Allocate a string for the input data inside wasm module
            let input_data_wasm_ptr = self.send_byte_array(&input_data);

            let result = self.implementation.call(
                &self.store,
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
                            error!(
                                "Return length from wasm function of {} exceed maximum allowed",
                                result_length
                            );
                            (None, true)
                        } else if let Ok(result_data) =
                            memory_ref.get(input_data_wasm_ptr, result_length as usize)
                        {
                            if let Ok((result, run_again)) =
                                serde_json::from_slice(result_data.as_slice())
                            {
                                (result, run_again)
                            } else {
                                (None, true)
                            }
                        } else {
                            error!("could not get() memory_reference");
                            (None, true)
                        }
                    }
                    _ => {
                        error!("Unexpected return value from wasm function on invoke_export()");
                        (None, true)
                    }
                },
                Ok(None) => {
                    error!(
                        "None value returned by Wasm implementation.call(): {:?}",
                        self.source_url
                    );
                    error!("Inputs:\n{:?}", inputs);
                    (None, true)
                }
                Err(err) => {
                    error!(
                        "Error returned by Wasm implementation.call() on '{}': {:?}",
                        self.source_url, err
                    );
                    error!("Inputs:\n{:?}", inputs);
                    (None, true)
                }
            };
        }

        (None, true)
    }
}

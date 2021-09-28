use anyhow::Result as AnyhowResult;
use flowcore::lib_provider::Provider;
use flowcore::{Implementation, RunAgain};
use log::{error, info, trace};
use serde_json::Value;
use std::sync::{Arc, Mutex};
use url::Url;
use wasmtime::*;

use crate::errors::*;

const DEFAULT_WASM_FILENAME: &str = "module";

const MAX_RESULT_SIZE: i32 = 1024;

#[derive(Debug)]
pub struct WasmExecutor {
    store: Arc<Mutex<Store<()>>>,
    memory: Memory,
    instance: Instance,
    implementation: Func,
    alloc: Func,
    source_url: Url,
}

impl WasmExecutor {
    pub fn new(
        store: Store<()>,
        memory: Memory,
        instance: Instance,
        implementation: Func,
        alloc: Func,
        source_url: &Url,
    ) -> Self {
        WasmExecutor {
            store: Arc::new(Mutex::new(store)),
            memory,
            instance,
            implementation,
            alloc,
            source_url: source_url.clone(),
        }
    }

    // Serialize the inputs into JSON and then write them into the linear memory for WASM to read
    // Return the offset of the data in linear memory and the data size in bytes
    fn send_inputs(&self, inputs: &[Value], store: &mut Store<()>) -> Result<(i32, i32)> {
        let input_data = serde_json::to_vec(&inputs)?;
        let value_array = self
            .alloc(MAX_RESULT_SIZE, store)
            .map_err(|_| "Could not call WASM alloc() function")?;
        match value_array[0] {
            Val::I32(offset) => {
                self.memory
                    .write(store, offset as usize, &input_data)
                    .map_err(|_| "Could not write to WASM Linear Memory")?;
                Ok((offset as i32, input_data.len() as i32))
            }
            _ => bail!("Unexpected return type from WASM alloc() function"),
        }
    }

    fn alloc(&self, length: i32, store: &mut Store<()>) -> AnyhowResult<Box<[Val]>> {
        self.alloc.call(store, &[Val::I32(length)])
    }

    fn call(&self, offset: i32, length: i32, store: &mut Store<()>) -> AnyhowResult<Box<[Val]>> {
        self.implementation
            .call(store, &[Val::I32(offset), Val::I32(length)])
    }

    fn get_result(
        &self,
        result: AnyhowResult<Box<[Val]>>,
        offset: usize,
        store: &mut Store<()>,
    ) -> (Option<Value>, RunAgain) {
        return match result {
            Ok(value) => match *value {
                [Val::I32(result_length)] => {
                    trace!("Return length from wasm function of {}", result_length);
                    if result_length > MAX_RESULT_SIZE {
                        error!(
                            "Return length from wasm function of {} exceed maximum allowed",
                            result_length
                        );
                        return (None, true);
                    }

                    let mut buffer = Vec::<u8>::with_capacity(MAX_RESULT_SIZE as usize);
                    if self.memory.read(store, offset, &mut buffer).is_err() {
                        error!("could not read return value from WASM linear memory");
                        return (None, true);
                    }

                    match serde_json::from_slice(&buffer) {
                        Ok((result, run_again)) => (result, run_again),
                        _ => {
                            error!("could not deserialize json response from WASM");
                            (None, true)
                        }
                    }
                }
                _ => {
                    error!("Unexpected return type from WASM implementation.call()");
                    (None, true)
                }
            },
            Err(err) => {
                error!(
                    "Error returned by WASM implementation.call() on '{}': {:?}",
                    self.source_url, err
                );
                (None, true)
            }
        };
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
    let module = Module::new(&engine, content).map_err(|_| "Could not create WASM Module")?;
    let mut store: Store<()> = Store::default();
    let instance =
        Instance::new(&mut store, &module, &[]).map_err(|_| "Could not create WASM Instance")?;
    let memory = instance
        .get_memory(&mut store, "memory")
        .ok_or("Could not get WASM linear memory")?;
    let implementation = instance
        .get_func(&mut store, "run_wasm")
        .ok_or("Could not get the WASM instance() function")?;

    // TODO get types function
    let alloc = instance
        .get_func(&mut store, "alloc")
        .ok_or("Could not get the WASM alloc() function")?;

    info!("Loaded wasm module from: '{}'", source_url);

    Ok(WasmExecutor::new(
        store,
        memory,
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
        if let Ok(mut store) = self.store.lock() {
            if let Ok((offset, length)) = self.send_inputs(inputs, &mut store) {
                let result = self.call(offset, length, &mut store);
                return self.get_result(result, offset as usize, &mut store);
            }
        }

        error!("Could not lock WASM store");
        (None, true)
    }
}

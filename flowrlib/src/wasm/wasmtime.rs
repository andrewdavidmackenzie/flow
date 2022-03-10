use anyhow::Result as AnyhowResult;
use flowcore::{Implementation, RunAgain};
use flowcore::errors::*;
use flowcore::lib_provider::Provider;
use log::info;
use log::trace;
use serde_json::Value;
use std::sync::{Arc, Mutex};
use url::Url;
use wasmtime::*;

const DEFAULT_WASM_FILENAME: &str = "module";

const MAX_RESULT_SIZE: i32 = 1024;

#[derive(Debug)]
pub struct WasmExecutor {
    store: Arc<Mutex<Store<()>>>,
    memory: Memory,
    implementation: Func,
    alloc: Func,
    source_url: Url,
}

impl WasmExecutor {
    pub fn new(
        store: Store<()>,
        memory: Memory,
        implementation: Func,
        alloc: Func,
        source_url: &Url,
    ) -> Self {
        WasmExecutor {
            store: Arc::new(Mutex::new(store)),
            memory,
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
    ) -> Result<(Option<Value>, RunAgain)> {
        return match result {
            Ok(value) => match *value {
                [Val::I32(result_length)] => {
                    trace!("Return length from wasm function of {}", result_length);
                    if result_length > MAX_RESULT_SIZE {
                        bail!(
                            "Return length from wasm function of {} exceed maximum allowed",
                            result_length
                        );
                    }

                    let mut buffer = vec![0u8; result_length as usize];
                    self
                        .memory
                        .read(store, offset, buffer.as_mut_slice())
                        .map_err(|_| "could not read return value from WASM linear memory")?;

                    match serde_json::from_slice(&buffer) {
                        Ok((result, run_again)) => Ok((result, run_again)),
                        Err(e) => bail!("could not deserialize json response from WASM: {}", e)
                    }
                }
                _ => {
                    bail!("Unexpected return type from WASM implementation.call()");
                }
            },
            Err(err) => {
                bail!(
                    "Error returned by WASM implementation.call() on '{}': {:?}",
                    self.source_url, err
                );
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

    let mut store: Store<()> = Store::default();
    let module = Module::new(store.engine(), content)
        .map_err(|e| format!("Could not create WASM Module: {}", e))?;
    let instance = Instance::new(&mut store, &module, &[])
        .map_err(|e| format!("Could not create WASM Instance: {}", e))?;
    let memory = instance
        .get_memory(&mut store, "memory")
        .ok_or("Could not get WASM linear memory")?;
    let implementation = instance
        .get_func(&mut store, "run_wasm")
        .ok_or("Could not get the WASM instance() function")?;

    // TODO get typed function
    let alloc = instance
        .get_func(&mut store, "alloc")
        .ok_or("Could not get the WASM alloc() function")?;

    info!("Loaded wasm module from: '{}'", source_url);

    Ok(WasmExecutor::new(
        store,
        memory,
        implementation,
        alloc,
        source_url,
    ))
}

unsafe impl Send for WasmExecutor {}

unsafe impl Sync for WasmExecutor {}

impl Implementation for WasmExecutor {
    fn run(&self, inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
        let mut store = self.store.lock().map_err(|_| "Could not lock WASM store")?;

        let (offset, length) = self.send_inputs(inputs, &mut store)?;

        let result = self.call(offset, length, &mut store);
        self.get_result(result, offset as usize, &mut store)
    }
}

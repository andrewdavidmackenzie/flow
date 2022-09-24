use std::cmp::max;
use std::sync::{Arc, Mutex};

use log::info;
use log::trace;
use serde_json::Value;
use url::Url;
use wasmtime::*;

use flowcore::{Implementation, RunAgain};
use flowcore::errors::*;
use flowcore::meta_provider::Provider;

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
    // Serialize the inputs into JSON and then write them into the linear memory for WASM to read
    // Return the offset of the data in linear memory and the data size in bytes
    fn send_inputs(&self, store: &mut Store<()>, inputs: &[Value]) -> Result<(i32, i32)> {
        let input_data = serde_json::to_vec(&inputs)?;
        let alloc_size = max(input_data.len() as i32, MAX_RESULT_SIZE);
        let offset = self.alloc(alloc_size, store)?;
        self.memory
            .write(store, offset as usize, &input_data)
            .map_err(|_| "Could not write to WASM Linear Memory")?;
        Ok((offset as i32, input_data.len() as i32))
    }

    // Call the "alloc" wasm function
    // - `length` is the length of block of memory to allocate
    // - returns the offset to the allocated memory
    fn alloc(&self, length: i32, store: &mut Store<()>) -> Result<i32> {
        let mut results: [Val;1] = [Val::I32(0)];
        let params = [Val::I32(length)];
        self.alloc.call(store, &params, &mut results)
            .map_err(|_| "WASM alloc() call failed")?;

        match results[0] {
            Val::I32(offset) => Ok(offset as i32),
            _ => bail!("WASM alloc() failed"),
        }
    }

    // Call the "implementation" wasm function
    // - `offset` is the offset to the input values (json), and the length of the json
    // - `length` is the length of the input json
    // - returns the length of the resulting json, at the same offset
    fn call(&self, offset: i32, length: i32, store: &mut Store<()>) -> Result<i32> {
        let mut results: [Val;1] = [Val::I32(0)];
        let params = [Val::I32(offset), Val::I32(length)];
        self.implementation
            .call(store, &params, &mut results)
            .map_err(|_| format!("Error returned by WASM implementation.call() for {:?}",
            self.source_url))?;

        match results[0] {
            Val::I32(result_length) => {
                trace!("Return length from wasm function of {}", result_length);
                if result_length > MAX_RESULT_SIZE {
                    bail!(
                    "Return length from wasm function of {} exceed maximum allowed",
                    result_length
                    );
                }
                Ok(result_length)
            },
            _ => bail!(format!("Unexpected value returned by WASM Func.call()()"))
        }
    }

    fn get_result(
        &self,
        result_length: i32,
        offset: usize,
        store: &mut Store<()>,
    ) -> Result<(Option<Value>, RunAgain)> {
        let mut buffer: Vec<u8> = vec![0u8; result_length as usize];
        self
            .memory
            .read(store, offset, &mut buffer)
            .map_err(|_| "could not read return value from WASM linear memory")?;

        let result_returned = serde_json::from_slice(buffer.as_slice())
            .chain_err(|| "Could not convert returned data from wasm to json")?;
        trace!("WASM run() function invocation Result = {:?}", result_returned);
        result_returned
    }
}


impl Implementation for WasmExecutor {
    fn run(&self, inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
        let mut store = self.store.lock().map_err(|_| "Could not lock WASM store")?;
        let (offset, length) = self.send_inputs(&mut store, inputs)?;
        let result_length = self.call(offset, length, &mut store)?;
        self.get_result(result_length, offset as usize, &mut store)
    }
}

unsafe impl Send for WasmExecutor {}

unsafe impl Sync for WasmExecutor {}

/// load a Wasm module from the specified Url and return it wrapped in a WasmExecutor `Implementation`
pub fn load(provider: &dyn Provider, source_url: &Url) -> Result<WasmExecutor> {
    let (resolved_url, _) = provider
        .resolve_url(source_url, DEFAULT_WASM_FILENAME, &["wasm"])
        .chain_err(|| format!("Could not resolve url '{}' for wasm file", source_url))?;
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

    Ok(WasmExecutor {
        store: Arc::new(Mutex::new(store)),
        memory,
        implementation,
        alloc,
        source_url: source_url.clone(),
    })
}

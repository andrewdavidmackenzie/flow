use std::cmp::max;
use std::sync::{Arc, Mutex};

use log::info;
use log::trace;
use serde_json::Value;
use url::Url;
use wasmtime::{Func, Instance, Memory, Module, Store, Val};

use flowcore::{Implementation, RunAgain};
use flowcore::errors::{Result, ResultExt, bail};
use flowcore::provider::Provider;

const DEFAULT_WASM_FILENAME: &str = "module";

const MAX_RESULT_SIZE: i32 = 1024;

#[derive(Debug)]
pub struct Executor {
    store: Arc<Mutex<Store<()>>>,
    memory: Memory,
    implementation: Func,
    alloc: Func,
    source_url: Url,
}

impl Executor {
    // Serialize the inputs into JSON and then write them into the linear memory for WASM to read
    // Return the offset of the data in linear memory and the data size in bytes
    fn send_inputs(&self, store: &mut Store<()>, inputs: &[Value]) -> Result<(i32, i32)> {
        let input_data = serde_json::to_vec(&inputs)?;
        let alloc_size = max(i32::try_from(input_data.len())?, MAX_RESULT_SIZE);
        let offset = self.alloc(alloc_size, store)?;
        self.memory
            .write(store, usize::try_from(offset)?, &input_data)
            .map_err(|_| "Could not write to WASM Linear Memory")?;
        let data_size = i32::try_from(input_data.len())?;
        Ok((offset, data_size))
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
            Val::I32(offset) => Ok(offset),
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
            .map_err(|e| format!("Error returned by WASM implementation.call() for {:?} => '{}'",
            self.source_url, e))?;

        match results[0] {
            Val::I32(result_length) => {
                trace!("Return length from wasm function of {result_length}");
                if result_length > MAX_RESULT_SIZE {
                    bail!(
                    "Return length from wasm function of {} exceed maximum allowed",
                    result_length
                    );
                }
                Ok(result_length)
            },
            _ => bail!("Unexpected value returned by WASM Func.call()()")
        }
    }

    fn get_result(
        &self,
        result_length: i32,
        offset: usize,
        store: &mut Store<()>,
    ) -> Result<(Option<Value>, RunAgain)> {
        assert!(result_length >= 0, "result_length was negative");
        #[allow(clippy::cast_sign_loss)]
        let mut buffer: Vec<u8> = vec![0u8; result_length as usize];
        self
            .memory
            .read(store, offset, &mut buffer)
            .map_err(|_| "could not read return value from WASM linear memory")?;

        let result_returned = serde_json::from_slice(buffer.as_slice())
            .chain_err(|| "Could not convert returned data from wasm to json")?;
        trace!("WASM run() function invocation Result = {result_returned:?}");
        result_returned
    }
}


impl Implementation for Executor {
    fn run(&self, inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
        let mut store = self.store.lock().map_err(|_| "Could not lock WASM store")?;
        let (offset, length) = self.send_inputs(&mut store, inputs)?;
        let result_length = self.call(offset, length, &mut store)?;
        assert!(offset >= 0, "offset was negative");
        #[allow(clippy::cast_sign_loss)]
        self.get_result(result_length, offset as usize, &mut store)
    }
}

/// load a Wasm module from the specified Url and return it wrapped in a `WasmExecutor` `Implementation`
pub fn load(provider: &Arc<dyn Provider>, source_url: &Url) -> Result<Executor> {
    trace!("Attempting to load WASM module from '{source_url}'");
    let (resolved_url, _) = provider
        .resolve_url(source_url, DEFAULT_WASM_FILENAME, &["wasm"])
        .chain_err(|| format!("Could not resolve url '{source_url}' for wasm file"))?;
    let content = provider.get_contents(&resolved_url).chain_err(|| {
        format!("Could not fetch content from url '{resolved_url}' for loading wasm")
    })?;

    let mut store: Store<()> = Store::default();
    let module = Module::from_binary(store.engine(), &content)
        .map_err(|e| format!("Could not create WASM Module: {e}"))?;
    let instance = Instance::new(&mut store, &module, &[])
        .map_err(|e| format!("Could not create WASM Instance: {e}"))?;
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

    info!("Loaded wasm module from: '{source_url}'");

    Ok(Executor {
        store: Arc::new(Mutex::new(store)),
        memory,
        implementation,
        alloc,
        source_url: source_url.clone(),
    })
}

#[cfg(test)]
mod test {
    use std::path::Path;
    use std::sync::Arc;

    use serde_json::json;
    use url::Url;

    use flowcore::content::file_provider::FileProvider;
    use flowcore::Implementation;
    use flowcore::provider::Provider;

    #[test]
    fn load_test_wasm() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/add.wasm");
        let url = Url::from_file_path(path).expect("Could not convert path to Url");
        let provider = Arc::new(FileProvider{}) as Arc<dyn Provider>;
        let adder = &super::load(&provider, &url)
            .expect("Could not load test_wasm.wasm") as &dyn Implementation;

        let inputs = vec![json!(1), json!(2)];
        let (value, run_again) = adder.run(&inputs).expect("Could not call run");

        assert_eq!(value, Some(json!(3)));
        assert!(run_again);
    }
}
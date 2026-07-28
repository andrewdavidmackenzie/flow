use std::cmp::max;
use std::sync::{Arc, Mutex};

use log::info;
use log::trace;
use serde_json::Value;
use url::Url;
use wasmtime::{Config, Engine, Func, Instance, Memory, Module, Store, Val};

use flowcore::bail;
use flowcore::errors::{Result, ResultExt};
use flowcore::provider::Provider;
use flowcore::{Implementation, RunAgain};

const DEFAULT_WASM_FILENAME: &str = "module";

const MAX_RESULT_SIZE: i32 = 256 * 1024;

#[derive(Debug)]
pub struct Executor {
    store: Arc<Mutex<Store<()>>>,
    memory: Memory,
    implementation: Func,
    alloc: Func,
    dealloc: Option<Func>,
    source_url: Url,
}

impl Executor {
    /// Serialize the inputs into JSON and write them into the WASM linear memory.
    ///
    /// Returns `(offset, data_size, alloc_size)`:
    /// - `offset`: where the data was written in linear memory
    /// - `data_size`: length of the serialized input data
    /// - `alloc_size`: total bytes allocated (for `dealloc` after use)
    fn send_inputs(&self, store: &mut Store<()>, inputs: &[Value]) -> Result<(i32, i32, i32)> {
        let input_data = serde_json::to_vec(&inputs)?;
        let input_len = input_data.len();
        let alloc_size = max(
            i32::try_from(input_len).map_err(|e| {
                format!(
                    "Input data size {} exceeds i32::MAX for WASM '{}': {e}",
                    input_len, self.source_url
                )
            })?,
            MAX_RESULT_SIZE,
        );
        let offset = self.alloc(alloc_size, store)?;
        self.memory
            .write(store, usize::try_from(offset)?, &input_data)
            .map_err(|_| "Could not write to WASM Linear Memory")?;
        let data_size = i32::try_from(input_data.len())?;
        Ok((offset, data_size, alloc_size))
    }

    // Call the "alloc" wasm function
    // - `length` is the length of block of memory to allocate
    // - returns the offset to the allocated memory
    fn alloc(&self, length: i32, store: &mut Store<()>) -> Result<i32> {
        let mut results: [Val; 1] = [Val::I32(0)];
        let params = [Val::I32(length)];
        self.alloc
            .call(store, &params, &mut results)
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
        let mut results: [Val; 1] = [Val::I32(0)];
        let params = [Val::I32(offset), Val::I32(length)];
        self.implementation
            .call(store, &params, &mut results)
            .map_err(|e| {
                format!(
                    "Error returned by WASM implementation.call() for {:?} => '{}'",
                    self.source_url, e
                )
            })?;

        match results[0] {
            Val::I32(result_length) => {
                trace!("Return length from wasm function of {result_length}");
                if result_length > MAX_RESULT_SIZE {
                    bail!(
                        "Return length from wasm function of {} exceeds maximum allowed",
                        result_length
                    );
                }
                Ok(result_length)
            }
            _ => bail!("Unexpected value returned by WASM Func.call()()"),
        }
    }

    /// Free a buffer previously allocated by `alloc` inside the WASM guest.
    /// This is a no-op if the module was compiled without a `dealloc` export
    /// (backward compatibility with older WASM modules).
    fn dealloc(&self, offset: i32, size: i32, store: &mut Store<()>) -> Result<()> {
        if let Some(ref dealloc_fn) = self.dealloc {
            let params = [Val::I32(offset), Val::I32(size)];
            let mut results: [Val; 0] = [];
            dealloc_fn
                .call(store, &params, &mut results)
                .map_err(|e| format!("WASM dealloc() call failed: {e}"))?;
        }
        Ok(())
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
        self.memory
            .read(store, offset, &mut buffer)
            .map_err(|_| "could not read return value from WASM linear memory")?;

        let result_returned = serde_json::from_slice(buffer.as_slice())
            .chain_err(|| "Could not convert returned data from wasm to json")?;
        trace!("WASM run() function invocation Result = {result_returned:?}");
        result_returned
    }
}

/// RAII guard that calls `track_execution_end` on drop,
/// ensuring the counter is decremented even if execution fails.
struct ExecutionGuard;

impl ExecutionGuard {
    fn new() -> Self {
        super::executor::track_execution_start();
        Self
    }
}

impl Drop for ExecutionGuard {
    fn drop(&mut self) {
        super::executor::track_execution_end();
    }
}

impl Implementation for Executor {
    fn run(&self, inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
        let mut store = self.store.lock().map_err(|_| "Could not lock WASM store")?;
        // Track execution AFTER acquiring the Mutex — so we count actual
        // execution, not time spent waiting for the lock.
        // The guard ensures the counter is decremented even if execution fails.
        let _guard = ExecutionGuard::new();
        let (offset, data_size, alloc_size) = self.send_inputs(&mut store, inputs)?;
        let result_length = self.call(offset, data_size, &mut store)?;
        assert!(offset >= 0, "offset was negative");
        #[allow(clippy::cast_sign_loss)]
        let result = self.get_result(result_length, offset as usize, &mut store)?;
        // Free the buffer allocated by alloc() to prevent linear memory exhaustion
        self.dealloc(offset, alloc_size, &mut store)?;
        Ok(result)
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

    let mut config = Config::new();
    config.max_wasm_stack(2 * 1024 * 1024);
    let engine = Engine::new(&config).map_err(|e| format!("Could not create WASM Engine: {e}"))?;
    let mut store: Store<()> = Store::new(&engine, ());
    let module = Module::from_binary(&engine, &content)
        .map_err(|e| format!("Could not create WASM Module: {e}"))?;
    let instance = Instance::new(&mut store, &module, &[])
        .map_err(|e| format!("Could not create WASM Instance: {e}"))?;
    let memory = instance
        .get_memory(&mut store, "memory")
        .ok_or("Could not get WASM linear memory")?;
    let implementation = instance
        .get_func(&mut store, "run_wasm")
        .ok_or("Could not get the WASM instance() function")?;

    let alloc = instance
        .get_func(&mut store, "alloc")
        .ok_or("Could not get the WASM alloc() function")?;
    let dealloc = instance.get_func(&mut store, "dealloc");

    info!("Loaded wasm module from: '{source_url}'");

    Ok(Executor {
        store: Arc::new(Mutex::new(store)),
        memory,
        implementation,
        alloc,
        dealloc,
        source_url: source_url.clone(),
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {
    use std::path::Path;
    use std::sync::Arc;

    use serde_json::json;
    use url::Url;

    use flowcore::content::file_provider::FileProvider;
    use flowcore::provider::Provider;
    use flowcore::Implementation;

    fn load_adder() -> Box<dyn Implementation> {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("add.wasm");
        let url = Url::from_file_path(path).expect("Could not convert path to Url");
        let provider = Arc::new(FileProvider {}) as Arc<dyn Provider>;
        Box::new(super::load(&provider, &url).expect("Could not load add.wasm"))
    }

    #[test]
    fn load_test_wasm() {
        let adder = load_adder();

        let inputs = vec![json!(1), json!(2)];
        let (value, run_again) = adder.run(&inputs).expect("Could not call run");

        assert_eq!(value, Some(json!(3)));
        assert!(run_again);
    }

    /// Verify that a WASM executor can be called many times without exhausting
    /// linear memory. Each call allocates at least `MAX_RESULT_SIZE` (256KB) via
    /// the guest `alloc` function. Without a corresponding `dealloc`, repeated
    /// calls will eventually overflow the i32 address space (~2GB) and cause
    /// `TryFromIntError`.
    ///
    /// See: <https://github.com/andrewdavidmackenzie/flow/issues/2948>
    #[test]
    fn repeated_wasm_calls_do_not_exhaust_memory() {
        let adder = load_adder();
        let inputs = vec![json!(1), json!(2)];

        // Each alloc leaks MAX_RESULT_SIZE (256KB) of WASM linear memory.
        // After ~8192 calls the cumulative offset exceeds i32::MAX (~2GB),
        // causing alloc to return a pointer that cannot be represented as a
        // positive i32 offset. Use 10_000 iterations for margin.
        let iterations = 10_000;

        for i in 0..iterations {
            let (value, run_again) = adder
                .run(&inputs)
                .unwrap_or_else(|e| panic!("WASM call failed on iteration {i}: {e}"));
            assert_eq!(value, Some(json!(3)), "Wrong result on iteration {i}");
            assert!(run_again, "run_again was false on iteration {i}");
        }
    }

    /// Verify that alloc+dealloc keeps offsets bounded.
    /// Without dealloc, each 256KB allocation pushes the offset higher
    /// until it wraps past `i32::MAX` after ~8192 calls.
    /// With dealloc, the guest allocator reclaims memory and offsets
    /// stay bounded regardless of iteration count.
    ///
    /// See: <https://github.com/andrewdavidmackenzie/flow/issues/2948>
    #[test]
    fn wasm_alloc_dealloc_offsets_stay_bounded() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("add.wasm");
        let url = Url::from_file_path(path).expect("Could not convert path to Url");
        let provider = Arc::new(FileProvider {}) as Arc<dyn Provider>;
        let executor = super::load(&provider, &url).expect("Could not load add.wasm");

        let mut store = executor.store.lock().unwrap();

        // 10_000 iterations × 256KB would exceed 2GB without dealloc
        let mut max_offset: i32 = 0;
        let iterations = 10_000;
        for i in 0..iterations {
            let offset = executor
                .alloc(super::MAX_RESULT_SIZE, &mut store)
                .unwrap_or_else(|e| panic!("alloc failed on iteration {i}: {e}"));
            assert!(
                offset >= 0,
                "alloc returned negative offset {offset} on iteration {i} \
                 (linear memory exhausted)"
            );
            if offset > max_offset {
                max_offset = offset;
            }
            executor
                .dealloc(offset, super::MAX_RESULT_SIZE, &mut store)
                .unwrap_or_else(|e| panic!("dealloc failed on iteration {i}: {e}"));
        }
        // With dealloc, the allocator reuses freed memory and max_offset
        // stays well below 2GB regardless of how many iterations we run.
        // Allow some allocator overhead (dlmalloc metadata, alignment, etc).
        let ten_mb = 10 * 1024 * 1024;
        assert!(
            max_offset < ten_mb,
            "max_offset {max_offset} ({:.1} MB) is unexpectedly large — \
             dealloc may not be freeing memory",
            f64::from(max_offset) / (1024.0 * 1024.0)
        );
    }
}

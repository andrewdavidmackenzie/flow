use std::cell::RefCell;
use std::cmp::max;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::sync::Arc;

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

/// Thread-local WASM execution state. Each executor thread gets its own
/// Store/Instance pair, avoiding Mutex contention.
struct ThreadLocalWasm {
    store: Store<()>,
    memory: Memory,
    implementation: Func,
    alloc: Func,
    dealloc: Option<Func>,
}

impl ThreadLocalWasm {
    /// Create a new thread-local WASM instance from a compiled module
    fn new(engine: &Engine, module: &Module) -> Result<Self> {
        let mut store = Store::new(engine, ());
        let instance = Instance::new(&mut store, module, &[])
            .map_err(|e| format!("Could not create WASM Instance: {e}"))?;
        let memory = instance
            .get_memory(&mut store, "memory")
            .ok_or("Could not get WASM linear memory")?;
        let implementation = instance
            .get_func(&mut store, "run_wasm")
            .ok_or("Could not get the WASM run_wasm() function")?;
        let alloc = instance
            .get_func(&mut store, "alloc")
            .ok_or("Could not get the WASM alloc() function")?;
        let dealloc = instance.get_func(&mut store, "dealloc");

        Ok(Self {
            store,
            memory,
            implementation,
            alloc,
            dealloc,
        })
    }

    /// Serialize the inputs into JSON and write them into the WASM linear memory.
    ///
    /// Returns `(offset, data_size, alloc_size)`:
    /// - `offset`: where the data was written in linear memory
    /// - `data_size`: length of the serialized input data
    /// - `alloc_size`: total bytes allocated (for `dealloc` after use)
    fn send_inputs(&mut self, inputs: &[Value], source_url: &Url) -> Result<(i32, i32, i32)> {
        let input_data = serde_json::to_vec(&inputs)?;
        let input_len = input_data.len();
        let alloc_size =
            max(
                i32::try_from(input_len).map_err(|e| {
                    format!(
                        "Input data size {input_len} exceeds i32::MAX for WASM '{source_url}': {e}",
                    )
                })?,
                MAX_RESULT_SIZE,
            );
        let offset = self.alloc_mem(alloc_size)?;
        self.memory
            .write(&mut self.store, usize::try_from(offset)?, &input_data)
            .map_err(|_| "Could not write to WASM Linear Memory")?;
        let data_size = i32::try_from(input_data.len())?;
        Ok((offset, data_size, alloc_size))
    }

    fn alloc_mem(&mut self, length: i32) -> Result<i32> {
        let mut results: [Val; 1] = [Val::I32(0)];
        let params = [Val::I32(length)];
        self.alloc
            .call(&mut self.store, &params, &mut results)
            .map_err(|e| format!("WASM alloc() call failed: {e}"))?;

        match results[0] {
            Val::I32(offset) => Ok(offset),
            _ => bail!("WASM alloc() failed"),
        }
    }

    fn call(&mut self, offset: i32, length: i32, source_url: &Url) -> Result<i32> {
        let mut results: [Val; 1] = [Val::I32(0)];
        let params = [Val::I32(offset), Val::I32(length)];
        self.implementation
            .call(&mut self.store, &params, &mut results)
            .map_err(|e| {
                format!("Error returned by WASM implementation.call() for {source_url:?} => '{e}'")
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
    fn dealloc_mem(&mut self, offset: i32, size: i32) -> Result<()> {
        if let Some(ref dealloc_fn) = self.dealloc {
            let params = [Val::I32(offset), Val::I32(size)];
            let mut results: [Val; 0] = [];
            dealloc_fn
                .call(&mut self.store, &params, &mut results)
                .map_err(|e| format!("WASM dealloc() call failed: {e}"))?;
        }
        Ok(())
    }

    fn get_result(
        &mut self,
        result_length: i32,
        offset: usize,
    ) -> Result<(Option<Value>, RunAgain)> {
        if result_length < 0 {
            bail!("WASM function returned negative result length: {result_length}");
        }
        #[allow(clippy::cast_sign_loss)]
        let mut buffer: Vec<u8> = vec![0u8; result_length as usize];
        self.memory
            .read(&mut self.store, offset, &mut buffer)
            .map_err(|_| "could not read return value from WASM linear memory")?;

        let result_returned = serde_json::from_slice(buffer.as_slice())
            .chain_err(|| "Could not convert returned data from wasm to json")?;
        trace!("WASM run() function invocation Result = {result_returned:?}");
        result_returned
    }

    fn run(&mut self, inputs: &[Value], source_url: &Url) -> Result<(Option<Value>, RunAgain)> {
        let (offset, data_size, alloc_size) = self.send_inputs(inputs, source_url)?;

        // Run the WASM function and read the result. Always free the allocated
        // buffer afterwards, even if call() or get_result() fails, to prevent
        // linear memory exhaustion on repeated errors.
        let run_result = self
            .call(offset, data_size, source_url)
            .and_then(|result_length| {
                if offset < 0 {
                    bail!("WASM alloc returned negative offset: {offset}");
                }
                #[allow(clippy::cast_sign_loss)]
                self.get_result(result_length, offset as usize)
            });

        // Free the buffer allocated by alloc() — ignore dealloc errors if the
        // main operation already failed (the original error is more useful).
        let dealloc_result = self.dealloc_mem(offset, alloc_size);

        match run_result {
            Ok(result) => {
                dealloc_result?;
                Ok(result)
            }
            Err(e) => Err(e),
        }
    }
}

/// A WASM implementation that uses thread-local Store instances for parallel execution.
///
/// The compiled `Engine` and `Module` are shared (thread-safe). Each executor thread
/// lazily creates its own `Store`/`Instance` on the first WASM job it receives,
/// avoiding Mutex contention.
#[derive(Debug)]
pub struct Executor {
    engine: Engine,
    module: Module,
    source_url: Url,
}

// Thread-local storage for per-thread WASM instances keyed by source URL
thread_local! {
    static THREAD_WASM: RefCell<HashMap<Url, ThreadLocalWasm>> =
        RefCell::new(HashMap::new());
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
        let _guard = ExecutionGuard::new();

        THREAD_WASM.with(|cell| {
            let mut map = cell.borrow_mut();

            // Lazily create a thread-local Store/Instance for this WASM module
            let tl = match map.entry(self.source_url.clone()) {
                Entry::Occupied(o) => o.into_mut(),
                Entry::Vacant(v) => {
                    let tl = ThreadLocalWasm::new(&self.engine, &self.module).chain_err(|| {
                        format!(
                            "Failed to create thread-local WASM instance for '{}'",
                            self.source_url
                        )
                    })?;
                    trace!(
                        "Created thread-local WASM instance for '{}'",
                        self.source_url
                    );
                    v.insert(tl)
                }
            };

            tl.run(inputs, &self.source_url)
        })
    }
}

/// Load a WASM module from the specified URL and return it as an `Implementation`.
///
/// The compiled `Engine` and `Module` are stored in the returned `Executor`.
/// Each executor thread will lazily create its own `Store`/`Instance` from the
/// shared Module on first use, enabling true parallel WASM execution.
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
    let module = Module::from_binary(&engine, &content)
        .map_err(|e| format!("Could not create WASM Module: {e}"))?;

    info!("Loaded wasm module from: '{source_url}'");

    Ok(Executor {
        engine,
        module,
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

    /// Verify that multiple threads can execute the same WASM module concurrently.
    /// Each thread lazily creates its own Store/Instance via thread-local storage.
    #[test]
    fn multi_threaded_wasm_execution() {
        use std::thread;

        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("add.wasm");
        let url = Url::from_file_path(path).expect("Could not convert path to Url");
        let provider = Arc::new(FileProvider {}) as Arc<dyn Provider>;
        let executor = Arc::new(super::load(&provider, &url).expect("Could not load add.wasm"));

        let num_threads = 4;
        let jobs_per_thread = 100;

        let handles: Vec<_> = (0..num_threads)
            .map(|t| {
                let exec = Arc::clone(&executor);
                thread::spawn(move || {
                    let inputs = vec![json!(t), json!(1)];
                    for j in 0..jobs_per_thread {
                        let (value, run_again) = exec
                            .run(&inputs)
                            .unwrap_or_else(|e| panic!("Thread {t} job {j} failed: {e}"));
                        assert_eq!(value, Some(json!(t + 1)), "Thread {t} job {j} wrong result");
                        assert!(run_again, "Thread {t} job {j} run_again was false");
                    }
                })
            })
            .collect();

        for (i, h) in handles.into_iter().enumerate() {
            h.join()
                .unwrap_or_else(|e| panic!("Thread {i} panicked: {e:?}"));
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

        // Create a ThreadLocalWasm directly for testing alloc/dealloc
        let mut tl =
            super::ThreadLocalWasm::new(&executor.engine, &executor.module).expect("new failed");

        // 10_000 iterations × 256KB would exceed 2GB without dealloc
        let mut max_offset: i32 = 0;
        let iterations = 10_000;
        for i in 0..iterations {
            let offset = tl
                .alloc_mem(super::MAX_RESULT_SIZE)
                .unwrap_or_else(|e| panic!("alloc failed on iteration {i}: {e}"));
            assert!(
                offset >= 0,
                "alloc returned negative offset {offset} on iteration {i} \
                 (linear memory exhausted)"
            );
            if offset > max_offset {
                max_offset = offset;
            }
            tl.dealloc_mem(offset, super::MAX_RESULT_SIZE)
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

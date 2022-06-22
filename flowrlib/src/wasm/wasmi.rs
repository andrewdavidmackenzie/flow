use flowcore::{Implementation, RunAgain};
use flowcore::errors::*;
use flowcore::meta_provider::Provider;
use log::{info, trace};
use serde_json::Value;
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

// Call the "alloc" wasm function
// - input parameter is the length of block of memory to allocate
// - returns the offset to the allocated memory
fn alloc(length: i32, instance: &ModuleRef) -> Result<i32> {
    match instance.invoke_export("alloc", &[RuntimeValue::I32(length)], &mut NopExternals) {
        Ok(Some(RuntimeValue::I32(offset))) => Ok(offset as i32),
        _ => bail!("Could not allocate memory in WASM with alloc()")
    }
}

// Serialize the inputs into JSON and then write them into the linear memory for WASM to read
// Return the offset of the data in linear memory and the data size in bytes
fn send_inputs(instance: &ModuleRef, memory: &MemoryRef, inputs: &[Value]) -> Result<(i32, i32)> {
    let bytes: &[u8] = &serde_json::to_vec(&inputs)?;
    let offset = alloc(bytes.len() as i32, instance)?;
    memory.set(offset as u32, bytes).chain_err(|| "Could not set memory")?;
    Ok((offset, bytes.len() as i32))
}

impl Implementation for WasmExecutor {
    fn run(&self, inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
        let module_ref = self.module.lock().map_err(|_| "Could not lock WASM module")?;
        let memory_ref = self.memory.lock().map_err(|_| "Could not lock WASM memory")?;

        let (offset, length) = send_inputs(&module_ref, &memory_ref, inputs)?;

        let result = module_ref.invoke_export(
            "run_wasm",
            &[
                RuntimeValue::I32(offset as i32),
                RuntimeValue::I32(length),
            ],
            &mut NopExternals,
        );

        match result {
            Ok(Some(RuntimeValue::I32(result_length))) => {
                trace!("Return length from wasm function of {}", result_length);
                if result_length > MAX_RESULT_SIZE {
                    bail!(
                        "Return length from wasm function of {} exceed maximum allowed",
                        result_length
                    );
                }

                let mut buffer: Vec<u8> = vec![0x0; result_length as usize];
                memory_ref.get_into(offset as u32, &mut buffer)
                    .chain_err(|| "Could not read wasm memory into owned slice")?;
                let result_returned = serde_json::from_slice(buffer.as_slice())
                    .chain_err(|| "Could not convert returned data from wasm to json")?;
                trace!("WASM run() function invocation Result = {:?}", result_returned);
                result_returned
            },
            Ok(_) => {
                bail!(format!("Unexpected value returned by Wasm invoke_export(): {:?}\nInputs:\n{:?}",
                    self.source_url, inputs));
            }
            Err(err) => {
                bail!("Error returned by Wasm invoke_export() on '{}': {:?}\nInputs:\n{:?}",
                    self.source_url, err, inputs);
            }
        }
    }
}

/// load a Wasm module from the specified Url.
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

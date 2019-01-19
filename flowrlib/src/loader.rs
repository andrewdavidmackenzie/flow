use std::sync::{Arc, Mutex};

use super::implementation_table::ImplementationLocatorTable;
use super::implementation_table::ImplementationLocator::Native;
use super::implementation_table::ImplementationLocator::Wasm;
use super::process::Process;
use super::manifest::Manifest;
use wasm_implementation::WasmImplementation;
use provider::Provider;
use url::Url;

pub struct Loader<'a> {
    global_lib_table: ImplementationLocatorTable<'a>
}

impl<'a> Loader<'a> {
    pub fn new() -> Self {
        Loader {
            global_lib_table: ImplementationLocatorTable::new(),
        }
    }

    pub fn load_flow(&self, provider: &Provider, url: &Url)
                     -> Result<Vec<Arc<Mutex<Process<'a>>>>, String> {
        let manifest = Manifest::load(provider, url)?;

        let mut runnables = Vec::<Arc<Mutex<Process>>>::new();

        for mut process in manifest.processes {
            // find the implementation from the implementation_source in the process
            if let Some(ref source) = self.global_lib_table.get(process.implementation_source()) {
                match source {
                    Native(impl_reference) => process.set_implementation(*impl_reference),
                    Wasm(source_path) => process.set_implementation(
                        WasmImplementation::load(provider, source_path)?)
                }
            }

            runnables.push(Arc::new(Mutex::new(process)));
        }

        Ok(runnables)
    }

    // Add a library to the runtime by adding it's ImplementationLocatorTable to the global
    // table for this runtime, so that then when we try to load a flow that references functions
    // in the library, they can be found.
    pub fn load_lib(&mut self, lib_manifest: ImplementationLocatorTable<'a>) {
        self.global_lib_table.extend(lib_manifest);
    }
}
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

    pub fn load_flow(&self, provider: &Provider, manifest_url: &Url)
                     -> Result<Vec<Arc<Mutex<Process<'a>>>>, String> {
        let manifest = Manifest::load(provider, manifest_url)?;

        let mut runnables = Vec::<Arc<Mutex<Process>>>::new();

        for mut process in manifest.processes {
            // find the implementation from the implementation_source in the process
            if let Some(ref source) = self.global_lib_table.locators.get(process.implementation_source()) {
                match source {
                    Native(impl_reference) => process.set_implementation(*impl_reference),
                    Wasm(source) => {
                        let wasm_url = manifest_url.join(source)
                            .map_err(|_e| format!("URL join error when trying to fetch wasm from '{}'",
                                                  source))?;

                        process.set_implementation(
                            WasmImplementation::load(provider, &wasm_url)?)
                    }
                }
            }

            runnables.push(Arc::new(Mutex::new(process)));
        }

        Ok(runnables)
    }

    // Add a library to the runtime by adding it's ImplementationLocatorTable to the global
    // table for this runtime, so that then when we try to load a flow that references functions
    // in the library, they can be found.
    pub fn add_lib(&mut self, lib_manifest: ImplementationLocatorTable<'a>) {
        self.global_lib_table.locators.extend(lib_manifest.locators);
    }
}
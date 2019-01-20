use std::sync::{Arc, Mutex};

use super::implementation_table::ImplementationLocatorTable;
use super::implementation::Implementation;
use super::implementation_table::ImplementationLocator::Native;
use super::implementation_table::ImplementationLocator::Wasm;
use super::process::Process;
use super::manifest::Manifest;
use wasm_executor::WasmExecutor;
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

        // find in the library, or load the implementation required - as specified by the source
        for mut process in manifest.processes {
            let source_url = Url::parse(process.implementation_source())
                .map_err(|_| format!("Could not convert process implementation source '{}' to a valid Url",
                                     process.implementation_source()))?;
            match source_url.scheme() {
                "lib" => {
                    // Try and find the implementation referenced in the libraries already loaded
                    if let Some(ref locator) = self.global_lib_table.locators.get(process.implementation_source()) {
                        match locator {
                            Native(implementation) => process.set_implementation(*implementation),
                            _ => return Err(format!("Did not find Native wrapper for Wasm implementation '{}'",
                                                   process.implementation_source()))
                        }
                    }
                }
                "" => { // Assume file with a relative path to the route of the flow's manifest
                    /*
                    TODO get this to work for relative paths
                    let relative_path = process.implementation_source();
                    let full_path = &manifest_url.clone().join(relative_path).unwrap(); */
                    let full_path = &Url::parse(process.implementation_source())
                        .map_err(|_| format!("Could not convert the implementation path '{}' to a Url",
                                 process.implementation_source()))?;
                    // TODO optimize so we don't load the implementation multiple times?
                    let implementation = WasmExecutor::wrap_wasm(provider, full_path).unwrap();
                    process.set_implementation(implementation);
                }
                "http" | "https" | "file" => {
                    // TODO optimize so we don't load the implementation multiple times?
                    let implementation = WasmExecutor::wrap_wasm(provider, &source_url).unwrap();
                    process.set_implementation(implementation);
                }
                _ => return Err(format!("Unexpected Url scheme for implemenation source: '{}'",
                                        process.implementation_source()))
            };

            runnables.push(Arc::new(Mutex::new(process)));
        }

        Ok(runnables)
    }

    // Add a library to the runtime by adding it's ImplementationLocatorTable to the global
    // table for this runtime, so that then when we try to load a flow that references functions
    // in the library, they can be found.
    pub fn add_lib(&mut self, provider: &Provider,
                   lib_manifest: ImplementationLocatorTable<'a>,
                   ilt_url: &Url)
                   -> Result<(), String> {
        self.global_lib_table.locators.extend(lib_manifest.locators.into_iter()
            .map(|(route, locator)| {
                match locator {
                    Wasm(ref source) => {
                        // Reference to a wasm implementation being added. Wrap it with the Wasm
                        // Native Implementation and return that for use later on execution.
                        let wasm_url = ilt_url.clone().join(source).unwrap();
                        (route, Native(WasmExecutor::wrap_wasm(provider, &wasm_url).unwrap()
                        as &Implementation))
                    }
                    _ => (route, locator), // Reference to Native implementation being added
                }
            }));

        Ok(())
    }
}
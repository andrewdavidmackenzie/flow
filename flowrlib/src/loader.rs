use std::sync::{Arc, Mutex};

use implementation::Implementation;
use implementation_table::ImplementationLocatorTable;
use implementation_table::ImplementationLocator::Native;
use implementation_table::ImplementationLocator::Wasm;
use process::Process;
use manifest::Manifest;
use provider::Provider;
use std::collections::HashMap;
use wasm;
use url;

pub struct Loader {
    global_lib_implementations: HashMap<String, Arc<Implementation>>,
    pub processes: Vec<Arc<Mutex<Process>>>,
}

impl Loader {
    pub fn new() -> Self {
        Loader {
            global_lib_implementations: HashMap::<String, Arc<Implementation>>::new(),
            processes: Vec::<Arc<Mutex<Process>>>::new(),
        }
    }

    /*
        Load all the processes defined in a flow from it's manifest, and then find all the
        implementations required for each one to run.

        A flow is dynamically loaded, so none of the implementations it brings can be static,
        they must all be dynamically loaded from WASM. Each one will be wrapped in a
        Native "WasmExecutor" implementation to make it appear native.

        It may have processes that use Implementations in libraries. Those must have been
        loaded previously. They maybe Native or Wasm implementations, but the Wasm ones will
        have been wrapped in a Native "WasmExecutor" implementation to make it appear native.
        Thus, all library implementations found will be Native.
    */
    pub fn load_flow(&mut self, provider: &Provider, manifest_url: &str) -> Result<(), String> {
        let manifest = Manifest::load(provider, manifest_url)?;

        // find in the library, or load the implementation required - as specified by the source
        for mut process in manifest.processes {
            let source_url = process.implementation_source().to_string();
            let parts: Vec<_> = source_url.split(":").collect();
            match parts[0] {
                "lib" => { // Try and find the implementation in the libraries already loaded
                    match self.global_lib_implementations.get(process.implementation_source()) {
                        Some(implementation) => process.set_implementation(implementation.clone()),
                        None => return Err(format!("Did not find implementation for '{}'",
                                                   source_url))
                    }
                }

                /*** These below are not 'lib:' references - hence are supplied implementations ***/
                _ => {
                    let full_url = url::join(manifest_url,
                                             process.implementation_source());
                    let wasm_executor = wasm::load(provider,
                                                   &process.name().to_lowercase(),
                                                   &full_url)?;
                    process.set_implementation(wasm_executor as Arc<Implementation>);
                }
            }

            self.processes.push(Arc::new(Mutex::new(process)));
        }

        Ok(())
    }

    /*
        Add a library to the runtime by adding it's ImplementationLocatorTable to the global
        table for this runtime, so that then when we try to load a flow that references functions
        in the library, they can be found.
    */
    pub fn add_lib(&mut self, provider: &Provider,
                   lib_manifest: ImplementationLocatorTable,
                   ilt_url: &str)
                   -> Result<(), String> {
        for (route, locator) in lib_manifest.locators {
            // if we don't already have an implementation loaded for that route
            if self.global_lib_implementations.get(&route).is_none() {
                // create or find the implementation we need
                let implementation = match locator {
                    Wasm(wasm_source) => {
                        // Path to the wasm source could be relative to the URL where we loaded the ILT from
                        let wasm_url = url::join(ilt_url, &wasm_source.0);
                        // Wasm implementation being added. Wrap it with the Wasm Native Implementation
                        let wasm_executor = wasm::load(provider, &wasm_source.1,
                                                       &wasm_url)?;
                        wasm_executor as Arc<Implementation>
                    }

                    // Native implementation from Lib
                    Native(implementation) => implementation
                };
                self.global_lib_implementations.insert(route, implementation);
            }
        }

        Ok(())
    }
}
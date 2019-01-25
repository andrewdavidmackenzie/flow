use std::sync::{Arc, Mutex};

use implementation::Implementation;
use implementation_table::ImplementationLocatorTable;
use implementation_table::ImplementationLocator::Native;
use implementation_table::ImplementationLocator::Wasm;
use process::Process;
use manifest::Manifest;
use provider::Provider;
use wasm_executor::WasmExecutor;
use wasmi::{Module, ModuleInstance, ImportsBuilder};
use url::Url;
use std::collections::HashMap;
use std::rc::Rc;

pub struct Loader {
    global_lib_implementations: HashMap<String, Rc<Implementation>>,
    pub processes: Vec<Arc<Mutex<Process>>>,
}

impl Loader {
    pub fn new() -> Self {
        Loader {
            global_lib_implementations: HashMap::<String, Rc<Implementation>>::new(),
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
    pub fn load_flow(&mut self, provider: &Provider, manifest_url: &Url)
                     -> Result<(), String> {
        let manifest = Manifest::load(provider, manifest_url)?;

        // find in the library, or load the implementation required - as specified by the source
        for mut process in manifest.processes {
            let source_url = Url::parse(process.implementation_source())
                .map_err(|_| format!("Could not convert process implementation source '{}' to a valid Url",
                                     process.implementation_source()))?;

            match source_url.scheme() {
                "lib" => { // Try and find the implementation in the libraries already loaded
                    match self.global_lib_implementations.get(process.implementation_source()) {
                        Some(implementation) => process.set_implementation(implementation.clone()),
                        None => return Err(format!("Did not find implementation for '{}'", source_url))
                    }
                },
                /*
                                "" => { // If no scheme, assume a relative path to the route of the flow's manifest
                                    /*
                                    TODO get this to work for relative paths
                                    let relative_path = process.implementation_source();
                                    let full_path = &manifest_url.clone().join(relative_path).unwrap(); */
                                    let full_path = Url::parse(process.implementation_source())
                                        .map_err(|_| format!("Could not convert the implementation path '{}' to a Url",
                                                             process.implementation_source()))?;

                                    let wasm_executor = Self::load_wasm(&mut modules, provider, full_path)?;
                                    process.set_implementation(wasm_executor as Box<Implementation>);
                                }

                                "http" | "https" | "file" => {
                                    let wasm_executor = Self::load_wasm(&mut modules, provider, source_url)?;
                                    process.set_implementation(wasm_executor as Box<Implementation>);
                                }
                */

                _ => return Err(format!("Unexpected Url scheme for implemenation source: '{}'",
                                        process.implementation_source()))
            }

            self.processes.push(Arc::new(Mutex::new(process)));
        }

        Ok(())
    }

    /*
        load a Wasm module from the specified Url.
    */
    pub fn load_wasm(provider: &Provider, source_url: Url)
                     -> Result<Rc<WasmExecutor>, String> {
        let (resolved_url, _) = provider.resolve(&source_url)?;
        let content = provider.get(&resolved_url)?;

        let module = Module::from_buffer(content)
            .map_err(|e| e.to_string())?;

        let module_ref = Arc::new(ModuleInstance::new(&module,
                                                      &ImportsBuilder::default())
            .map_err(|e| e.to_string())?
            .assert_no_start());

        let executor = WasmExecutor { module: Arc::new(Mutex::new(module_ref.clone())) };

        Ok(Rc::new(executor))
    }


    /*
        Add a library to the runtime by adding it's ImplementationLocatorTable to the global
        table for this runtime, so that then when we try to load a flow that references functions
        in the library, they can be found.
    */
    pub fn add_lib(&mut self, provider: &Provider,
                   lib_manifest: ImplementationLocatorTable,
                   ilt_url: &Url)
                   -> Result<(), String> {
        for (route, locator) in lib_manifest.locators {
            // if we don't already have an implementation loaded for that route
            if self.global_lib_implementations.get(&route).is_none() {
                // create or find the implementation we need
                let implementation = match locator {
                    Wasm(wasm_source) => {
                        // Path to the wasm source could be relative to the URL where we loaded the ILT from
                        let wasm_url = ilt_url.clone().join(&wasm_source).unwrap();
                        // Wasm implementation being added. Wrap it with the Wasm Native Implementation
                        let wasm_executor = Self::load_wasm(provider, wasm_url)?;
                        wasm_executor as Rc<Implementation>
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
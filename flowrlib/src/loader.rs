use std::collections::HashMap;
use std::sync::Arc;

use implementation::Implementation;
use implementation_table::ImplementationLocator::Native;
use implementation_table::ImplementationLocator::Wasm;
use implementation_table::ImplementationLocatorTable;
use manifest::Manifest;
use provider::Provider;
use url;
use wasm;

pub struct Loader {
    global_lib_implementations: HashMap<String, Arc<Implementation>>,
}

impl Loader {
    pub fn new() -> Self {
        Loader {
            global_lib_implementations: HashMap::<String, Arc<Implementation>>::new(),
        }
    }

    /*
        Load all the processes defined in a manifest, and then find all the
        implementations required for function execution later.

        A flow is dynamically loaded, so none of the implementations it brings can be static,
        they must all be dynamically loaded from WASM. Each one will be wrapped in a
        Native "WasmExecutor" implementation to make it present the same interface as a native
        implementation.

        The runtime that (statically) links this library may provide some native implementations
        already, before this is called.

        It may have processes that use Implementations in libraries. Those must have been
        loaded previously. They maybe Native or Wasm implementations, but the Wasm ones will
        have been wrapped in a Native "WasmExecutor" implementation to make it appear native.
        Thus, all library implementations found will be Native.
    */
    pub fn load_manifest(&mut self, provider: &Provider, manifest_url: &str) -> Result<Manifest, String> {
        let mut manifest = Manifest::load(provider, manifest_url)?;

        Self::load_libraries(provider, &manifest)?;

        // Find the implementations for all functions in this flow
        self.resolve_implementations(&mut manifest, provider, manifest_url)?;

        Ok(manifest)
    }

    /*
        Load libraries references referenced in the manifest
    */
    pub fn load_libraries(provider: &Provider, manifest: &Manifest) -> Result<(), String> {
        for library_reference in &manifest.lib_references {
            let (resolved_url, _) = provider.resolve(&library_reference, "manifest.json")?;
            let _contents = provider.get(&resolved_url)?;
            // TODO load the library from it's manifest - loading the WASM implementations
        }

        Ok(())
    }

    pub fn resolve_implementations(&mut self, manifest: &mut Manifest, provider: &Provider,
                                   manifest_url: &str) -> Result<String, String> {
        // find in a library, or load the implementation required - as specified by the source
        for mut function in &mut manifest.functions {
            let source_url = function.implementation_source().to_string();
            let parts: Vec<_> = source_url.split(":").collect();
            match parts[0] {
                "lib" => { // Try and find the implementation in the libraries already loaded
                    match self.global_lib_implementations.get(function.implementation_source()) {
                        Some(implementation) => function.set_implementation(implementation.clone()),
                        None => return Err(format!("Did not find implementation for '{}'", source_url))
                    }
                }

                /*** These below are not 'lib:' references - hence are supplied implementations ***/
                _ => {
                    let full_url = url::join(manifest_url,
                                             function.implementation_source());
                    let wasm_executor = wasm::load(provider,
                                                   &function.name().to_lowercase(),
                                                   &full_url)?;
                    function.set_implementation(wasm_executor as Arc<Implementation>);
                }
            }
        }

        Ok("All implementations found".into())
    }

    /*
        Add a library to the runtime by adding it's ImplementationLocatorTable to the global
        table for this runtime, so that then when we try to load a flow that references functions
        in the library, they can be found.
    */
    pub fn add_lib(&mut self, provider: &Provider,
                   lib_manifest: ImplementationLocatorTable,
                   ilt_url: &str) -> Result<(), String> {
        for (route, locator) in lib_manifest.locators {
            // if we don't already have an implementation loaded for that route
            if self.global_lib_implementations.get(&route).is_none() {
                // create or find the implementation we need
                let implementation = match locator {
                    Wasm(wasm_source) => {
                        info!("Looking for wasm source: '{}'", wasm_source.0);
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
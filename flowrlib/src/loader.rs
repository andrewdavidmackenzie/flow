use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;

use flow_impl::Implementation;
use log::{debug, info, trace};

use crate::errors::*;
use crate::lib_manifest::{ImplementationLocator::Native, ImplementationLocator::Wasm, LibraryManifest};
use crate::manifest::Manifest;
use crate::provider::Provider;
use crate::url;
use crate::wasm;

/// A `Loader` is responsible for loading a `Flow` from it's `Manifest`, loading the required
/// libraries needed by the flow and keeping track of the `Function` `Implementations` that
///will be used to execute it.
pub struct Loader {
    loaded_lib_references: HashSet<String>,
    global_lib_implementations: HashMap<String, Arc<dyn Implementation>>,
}

impl Loader {
    /// Create a new `Loader`
    pub fn new() -> Self {
        Loader {
            loaded_lib_references: HashSet::<String>::new(),
            global_lib_implementations: HashMap::<String, Arc<dyn Implementation>>::new(),
        }
    }

    /// Load all the processes defined in a manifest, and then find all the
    /// implementations required for function execution later.
    ///
    /// A flow is dynamically loaded, so none of the implementations it brings can be static,
    /// they must all be dynamically loaded from WASM. Each one will be wrapped in a
    /// Native "WasmExecutor" implementation to make it present the same interface as a native
    /// implementation.
    ///
    /// The run-time that (statically) links this library may provide some native implementations
    /// already, before this is called.
    ///
    /// It may have processes that use Implementations in libraries. Those must have been
    /// loaded previously. They maybe Native or Wasm implementations, but the Wasm ones will
    /// have been wrapped in a Native "WasmExecutor" implementation to make it appear native.
    /// Thus, all library implementations found will be Native.
    pub fn load_manifest(&mut self, provider: &dyn Provider, flow_manifest_url: &str) -> Result<Manifest> {
        debug!("Loading flow manifest from '{}'", flow_manifest_url);
        let mut flow_manifest = Manifest::load(provider, flow_manifest_url)?;

        self.load_libraries(provider, &flow_manifest)?;

        // Find the implementations for all functions in this flow
        self.resolve_implementations(&mut flow_manifest, provider, flow_manifest_url)?;

        Ok(flow_manifest)
    }

    /// Load libraries references referenced in the flows manifest that are not already loaded
    pub fn load_libraries(&mut self, provider: &dyn Provider, manifest: &Manifest) -> Result<()> {
        debug!("Loading libraries used by the flow");
        for library_reference in &manifest.lib_references {
            info!("Attempting to load library reference '{}'", library_reference);
            if !self.loaded_lib_references.contains(library_reference) {
                let (lib_manifest, lib_manifest_url) = LibraryManifest::load(provider, library_reference)?;
                debug!("Loading library '{}' from '{}'", library_reference, lib_manifest_url);
                self.add_lib(provider, library_reference, lib_manifest, &lib_manifest_url)?;
            }
        }

        Ok(())
    }

    /// Resolve or "find" all the implementations of functions for a flow
    /// The `root_url` is the url of the manifest or the directory where the manifest is located
    /// and is used in resolving relative references to other files.
    pub fn resolve_implementations(&mut self, flow_manifest: &mut Manifest, provider: &dyn Provider,
                                   root_url: &str) -> Result<String> {
        debug!("Resolving implementations");
        // find in a library, or load the supplied implementation - as specified by the source
        for function in &mut flow_manifest.functions {
            let implementation_source_url = function.implementation_location().to_string();
            let parts: Vec<_> = implementation_source_url.split(":").collect();
            match parts[0] {
                "lib" => {
                    let implementation = self.global_lib_implementations.get(function.implementation_location())
                        .chain_err(|| format!("Did not find implementation for '{}'", implementation_source_url))?;
                    trace!("Found implementation for '{}'", function.implementation_location());
                    function.set_implementation(implementation.clone());
                }

                /*** These below are not 'lib:' references - hence are supplied implementations ***/
                _ => {
                    let full_url = url::join(root_url,
                                             function.implementation_location());
                    let wasm_executor = wasm::load(provider,
                                                   &full_url)?;
                    function.set_implementation(Arc::new(wasm_executor) as Arc<dyn Implementation>);
                }
            }
        }

        Ok("All implementations found".into())
    }

    /// Add a library to the run-time by adding it's ImplementationLocators from the manifest to the
    /// table for this run-time, so that then when we try to load a flow that references functions
    /// in the library, they can be found.
    pub fn add_lib(&mut self, provider: &dyn Provider,
                   lib_reference: &str,
                   lib_manifest: LibraryManifest,
                   lib_manifest_url: &str) -> Result<()> {
        debug!("Loading library '{}' from '{}'", lib_manifest.metadata.name, lib_manifest_url);
        self.loaded_lib_references.insert(lib_reference.to_string());
        for (reference, locator) in lib_manifest.locators {
            // if we don't already have an implementation loaded for that reference
            if self.global_lib_implementations.get(&reference).is_none() {
                // create or find the implementation we need
                let implementation = match locator {
                    Wasm(wasm_source_relative) => {
                        // Path to the wasm source could be relative to the URL where we loaded the manifest from
                        let wasm_url = url::join(lib_manifest_url, &wasm_source_relative);
                        debug!("Looking for wasm source: '{}'", wasm_url);
                        // Wasm implementation being added. Wrap it with the Wasm Native Implementation
                        let wasm_executor = wasm::load(provider, &wasm_url)?;
                        Arc::new(wasm_executor) as Arc<dyn Implementation>
                    }

                    // Native implementation from Lib
                    Native(implementation) => implementation
                };
                self.global_lib_implementations.insert(reference, implementation);
            }
        }

        info!("Library '{}' loaded.", lib_manifest.metadata.name);
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::Loader;

    #[test]
    fn test_create() {
        let _ = Loader::new();
    }
}
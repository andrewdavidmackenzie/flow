use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;

use log::{debug, info, trace};
use url::Url;

use flow_impl::Implementation;
use flowcore::lib_manifest::{
    ImplementationLocator::Native, ImplementationLocator::Wasm, LibraryManifest,
};
use flowcore::manifest::Manifest;
use provider::lib_provider::LibProvider;

use crate::errors::*;
use crate::wasm;

/// A `Loader` is responsible for loading a compiled `Flow` from it's `Manifest`, loading the required
/// libraries needed by the flow and keeping track of the `Function` `Implementations` that
/// will be used to execute it.
#[derive(Default)]
pub struct Loader {
    loaded_lib_references: HashSet<Url>,
    global_lib_implementations: HashMap<Url, Arc<dyn Implementation>>,
}

impl Loader {
    /// Create a new `Loader`
    pub fn new() -> Self {
        Loader {
            loaded_lib_references: HashSet::<Url>::new(),
            global_lib_implementations: HashMap::<Url, Arc<dyn Implementation>>::new(),
        }
    }

    pub fn get_lib_implementations(&self) -> &HashMap<Url, Arc<dyn Implementation>> {
        &self.global_lib_implementations
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
    pub fn load_manifest(
        &mut self,
        provider: &dyn LibProvider,
        flow_manifest_url: &Url,
    ) -> Result<Manifest> {
        debug!("Loading flow manifest from '{}'", flow_manifest_url);
        let (mut flow_manifest, resolved_url) = Manifest::load(provider, flow_manifest_url)
            .chain_err(|| format!("Error while loading manifest from: '{}'", flow_manifest_url))?;

        self.load_libraries(provider, &flow_manifest)?;

        // Find the implementations for all functions in this flow
        self.resolve_implementations(&mut flow_manifest, &resolved_url, provider)?;

        Ok(flow_manifest)
    }

    /// Load libraries references referenced in the flows manifest that are not already loaded
    fn load_libraries(&mut self, provider: &dyn LibProvider, manifest: &Manifest) -> Result<()> {
        for library_implementation_reference in manifest.get_lib_references() {
            let mut lib_root_url = library_implementation_reference.clone();
            lib_root_url.set_path(""); // zero out the path

            // only load a library if it hasn't been loaded already
            if !self.loaded_lib_references.contains(&lib_root_url) {
                info!("Attempting to load library '{}'", lib_root_url);
                let (lib_manifest, lib_manifest_url) =
                    LibraryManifest::load(provider, &lib_root_url).chain_err(|| {
                        format!("Could not load library with root url: '{}'", lib_root_url)
                    })?;

                self.add_lib(provider, lib_manifest, &lib_manifest_url)?;
            }
        }

        Ok(())
    }

    /// Resolve or "find" all the implementations of functions for a flow
    /// The `root_url` is the url of the manifest or the directory where the manifest is located
    /// and is used in resolving relative references to other files.
    pub fn resolve_implementations(
        &mut self,
        flow_manifest: &mut Manifest,
        manifest_url: &Url,
        provider: &dyn LibProvider,
    ) -> Result<()> {
        debug!("Resolving implementations");
        // find in a library, or load the supplied implementation - as specified by the source
        for function in flow_manifest.get_functions() {
            let parts: Vec<_> = function.implementation_location().split(':').collect();
            match parts[0] {
                "lib" => {
                    let implementation_url = Url::parse(function.implementation_location())
                        .chain_err(|| {
                            "Could not create a Url from a lib: implementation location"
                        })?;
                    let implementation = self
                        .global_lib_implementations
                        .get(&implementation_url)
                        .chain_err(|| {
                        format!(
                            "Implementation at '{}' is not in loaded libraries",
                            function.implementation_location()
                        )
                    })?;
                    trace!(
                        "Found implementation location for '{}' in loaded libraries",
                        function.implementation_location()
                    );

                    // The implementation will have already been loaded when the library was loaded
                    // TODO move the actual loading of implementation in here so we lazy-load library
                    // implementations only when they are actually used by functions in the manifest

                    // Set the location of the implementation of this function
                    function.set_implementation(implementation.clone()); // Only clone of an Arc, not the object
                }

                /*** These below are not 'lib:' references - hence are supplied implementations ***/
                _ => {
                    let implementation_url = manifest_url
                        .join(function.implementation_location())
                        .map_err(|_| {
                            format!(
                                "Could not create supplied implementation url joining '{}' to manifest Url: {}",
                                function.implementation_location(), manifest_url
                            )
                        })?;
                    // load the actual implementation of the function
                    let wasm_executor = wasm::load(provider, &implementation_url)?;
                    function.set_implementation(Arc::new(wasm_executor) as Arc<dyn Implementation>);
                }
            }
        }

        Ok(())
    }

    /// Add a library to the run-time by adding it's ImplementationLocators from the manifest to the
    /// table for this run-time, so that then when we try to load a flow that references functions
    /// in the library, they can be found.
    pub fn add_lib(
        &mut self,
        provider: &dyn LibProvider,
        lib_manifest: LibraryManifest,
        lib_manifest_url: &Url,
    ) -> Result<()> {
        // Load all the implementations in the library
        for (implementation_reference, locator) in lib_manifest.locators {
            // if we don't already have an implementation loaded for that reference
            if self
                .global_lib_implementations
                .get(&implementation_reference)
                .is_none()
            {
                // create or find the implementation we need
                let implementation = match locator {
                    Wasm(wasm_source_relative) => {
                        // Path to the wasm source could be relative to the URL where we loaded the manifest from
                        let wasm_url = lib_manifest_url
                            .join(&wasm_source_relative)
                            .map_err(|e| e.to_string())?;
                        debug!("Looking for wasm source: '{}'", wasm_url);
                        // Wasm implementation being added. Wrap it with the Wasm Native Implementation
                        let wasm_executor = wasm::load(provider, &wasm_url)?;
                        Arc::new(wasm_executor) as Arc<dyn Implementation>
                    }

                    // Native implementation from Lib
                    Native(implementation) => implementation,
                };
                self.global_lib_implementations
                    .insert(implementation_reference, implementation);
            }
        }

        // track the fact we have already loaded this library
        self.loaded_lib_references.insert(lib_manifest_url.clone());
        info!("Loaded library: '{}'", lib_manifest_url);
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

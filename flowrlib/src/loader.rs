use std::collections::HashMap;
use std::sync::Arc;

use log::{debug, info, trace};
use url::Url;

use flowcore::errors::*;
use flowcore::Implementation;
use flowcore::meta_provider::Provider;
use flowcore::model::flow_manifest::FlowManifest;
use flowcore::model::lib_manifest::{
    ImplementationLocator::Native, ImplementationLocator::Wasm, LibraryManifest,
};

use crate::wasm;

/// A `Loader` is responsible for loading a compiled `Flow` from it's `Manifest`, loading the required
/// libraries needed by the flow and keeping track of the `Function` `Implementations` that
/// will be used to execute it.
#[derive(Default)]
pub struct Loader {
    /// HashMap of libraries already loaded. The key is the library reference Url
    /// (e.g. lib:://flowstdlib) and the entry is a tuple of the LibraryManifest
    /// and the resolved Url of where the manifest was read from
    loaded_libraries: HashMap<Url, (LibraryManifest, Url)>,
    loaded_implementations: HashMap<Url, Arc<dyn Implementation>>,
}

impl Loader {
    /// Create a new `Loader` that will be used to parse the manifest file create structs in
    /// memory ready for execution
    pub fn new() -> Self {
        Loader {
            loaded_libraries: HashMap::<Url, (LibraryManifest, Url)>::new(),
            loaded_implementations: HashMap::<Url, Arc<dyn Implementation>>::new(),
        }
    }

    /// Load all the functions defined in a manifest, and then find all the
    /// implementations required for function execution.
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
    pub fn load_flow(
        &mut self,
        provider: &dyn Provider,
        flow_manifest_url: &Url,
    ) -> Result<FlowManifest> {
        debug!("Loading flow manifest from '{}'", flow_manifest_url);
        let (mut flow_manifest, resolved_url) =
            FlowManifest::load(provider, flow_manifest_url)
                .chain_err(|| format!("Could not load manifest from: '{}'", flow_manifest_url))?;

        self.load_lib_implementations(provider, &flow_manifest)
            .chain_err(|| format!("Could not load libraries referenced by manifest at: {}",
                       resolved_url))?;

        // Find the implementations for all functions used in this flow
        self.resolve_function_implementations(provider, &mut flow_manifest, &resolved_url)
            .chain_err(|| "Could not resolve implementations required for flow execution")?;

        Ok(flow_manifest)
    }

    // Load the library manifest if is not already loaded
    fn load_lib_manifest_if_needed(
        &mut self,
        provider: &dyn Provider,
        lib_root_url: &Url,
    ) -> Result<()> {
        if self.loaded_libraries.get(lib_root_url).is_none() {
            info!("Attempting to load library '{}'", lib_root_url);
            let new_manifest_tuple =
                LibraryManifest::load(provider, lib_root_url).chain_err(|| {
                    format!("Could not load library with root url: '{}'", lib_root_url)
                })?;
            self.loaded_libraries
                .insert(lib_root_url.clone(), new_manifest_tuple);
        }
        Ok(())
    }

    // Get the tuple of the lib manifest and the url from where it was loaded from
    fn get_lib_manifest_tuple(
        &mut self,
        provider: &dyn Provider,
        lib_root_url: &Url,
    ) -> Result<(LibraryManifest, Url)> {
        self.load_lib_manifest_if_needed(provider, lib_root_url)?;

        let tuple = self
            .loaded_libraries
            .get(lib_root_url)
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Could not find (supposedly already loaded) library manifest",
                )
            })?;

        Ok(tuple.clone()) // TODO avoid the clone and return a reference
    }

    // Load libraries implementations referenced in the flow manifest
    fn load_lib_implementations(
        &mut self,
        provider: &dyn Provider,
        flow_manifest: &FlowManifest,
    ) -> Result<()> {
        for lib_reference in flow_manifest.get_lib_references() {
            // zero out the path to the implementation to get the root lib url
            let mut lib_root_url = lib_reference.clone();
            lib_root_url.set_path("");

            let manifest_tuple = self.get_lib_manifest_tuple(provider, &lib_root_url)?;

            self.load_lib_implementation(
                provider,
                lib_reference,
                &manifest_tuple,
            )?;
        }

        Ok(())
    }

    /// Add a library and all the implementations it contains to the loader by adding
    /// the ImplementationLocators, so that then when we try to load a flow that references functions
    /// in the library, they can be found.
    pub fn add_lib(
        &mut self,
        provider: &dyn Provider,
        lib_manifest: LibraryManifest,
        lib_manifest_url: &Url,
    ) -> Result<()> {
        if self
            .loaded_libraries
            .get(lib_manifest_url)
            .is_none()
        {
            let lib_manifest_tuple = (lib_manifest.clone(), lib_manifest_url.clone());

            // Load all the implementations in the library from their locators
            for (implementation_reference, locator) in lib_manifest.locators {
                // if we don't already have an implementation loaded for that reference
                if self
                    .loaded_implementations
                    .get(&implementation_reference)
                    .is_none()
                {
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
                    self.loaded_implementations
                        .insert(implementation_reference, implementation);
                }
            }

            // track the fact we have already loaded this library
            self.loaded_libraries
                .insert(lib_manifest_url.clone(), lib_manifest_tuple);
            info!("Loaded library: '{}'", lib_manifest_url);
        }

        Ok(())
    }

    // Add a library implementation to the loader
    fn load_lib_implementation(
        &mut self,
        provider: &dyn Provider,
        implementation_reference: &Url,
        lib_manifest_tuple: &(LibraryManifest, Url),
    ) -> Result<()> {
        // if we don't already have an implementation loaded for that reference
        if self
            .loaded_implementations
            .get(implementation_reference)
            .is_none()
        {
            let locator = lib_manifest_tuple
                .0
                .locators
                .get(implementation_reference)
                .ok_or(format!(
                    "Could not find ImplementationLocator for '{}' in library",
                    implementation_reference
                ))?;

            // find the implementation we need from the locator
            let implementation = match locator {
                Wasm(wasm_source_relative) => {
                    // Path to the wasm source could be relative to the URL where we loaded the manifest from
                    let wasm_url = lib_manifest_tuple
                        .1
                        .join(wasm_source_relative)
                        .map_err(|e| e.to_string())?;
                    debug!("Attempting to load wasm from source file: '{}'", wasm_url);
                    // Wasm implementation being added. Wrap it with the Wasm Native Implementation
                    let wasm_executor = wasm::load(provider, &wasm_url)?;
                    Arc::new(wasm_executor) as Arc<dyn Implementation>
                }
                Native(native_impl) => native_impl.clone(),
            };
            self.loaded_implementations
                .insert(implementation_reference.clone(), implementation);
        }

        Ok(())
    }

    // Find in previously loaded library functions or load a supplied implementation from file
    // all the implementations of functions used in a flow.
    // The `manifest_url` is used to resolve relative references to wasm source files.
    fn resolve_function_implementations(
        &mut self,
        provider: &dyn Provider,
        flow_manifest: &mut FlowManifest,
        manifest_url: &Url,
    ) -> Result<()> {
        debug!("Resolving implementations");
        // find in a library, or load the supplied implementation - as specified by the source
        for function in flow_manifest.get_functions() {
            let implementation =
                self.resolve_implementation(manifest_url, provider,function.implementation_location())?;
            function.set_implementation(implementation);
        }

        Ok(())
    }

    fn resolve_implementation(&mut self,
                                manifest_url: &Url,
                              provider: &dyn Provider,
                              implementation_location: &str)
        -> Result<Arc<dyn Implementation>> {
        return match implementation_location.split_once(':') {
            Some(("lib", _)) | Some(("context", _)) => {
                let implementation_url = Url::parse(implementation_location)
                    .chain_err(|| {
                        "Could not create a Url from the implementation location"
                    })?;
                let implementation = self
                    .loaded_implementations
                    .get(&implementation_url)
                    .ok_or_else(|| format!(
                        "Implementation at '{}' is not in loaded libraries",
                        implementation_location
                    ))?;
                // TODO ^^^^ this is where we would load a library function on demand
                trace!("\tFunction implementation loaded from '{}'", implementation_location);

                Ok(implementation.clone()) // Only clone of an Arc, not the object
            }

            // These below are not 'lib:' not 'context:' - hence are supplied implementations
            _ => {
                let implementation_url = manifest_url
                    .join(implementation_location)
                    .map_err(|_| {
                        format!(
                            "Could not create supplied implementation url joining '{}' to manifest Url: {}",
                            implementation_location, manifest_url
                        )
                    })?;
                // load the supplied implementation for the function from wasm file referenced
                let wasm_executor = wasm::load(provider, &implementation_url)?;
                Ok(Arc::new(wasm_executor) as Arc<dyn Implementation>)
            }
        }
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

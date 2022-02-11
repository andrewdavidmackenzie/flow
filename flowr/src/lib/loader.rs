use std::collections::HashMap;
use std::sync::Arc;

use log::{debug, info, trace};
use url::Url;

use flowcore::model::flow_manifest::FlowManifest;
use flowcore::Implementation;
use flowcore::model::lib_manifest::{
    ImplementationLocator::Native, ImplementationLocator::Wasm, LibraryManifest,
};
use flowcore::lib_provider::Provider;

use crate::errors::*;
use crate::wasm;

/// A `Loader` is responsible for loading a compiled `Flow` from it's `Manifest`, loading the required
/// libraries needed by the flow and keeping track of the `Function` `Implementations` that
/// will be used to execute it.
#[derive(Default)]
pub struct Loader {
    /// HashMap of libraries that have already had their manifests read. The key is the library
    /// reference Url (e.g. lib:://context) and the entry is a tuple of the LibraryManifest
    /// and the resolved Url of where the manifest was read from
    loaded_libraries_manifests: HashMap<Url, (LibraryManifest, Url)>,
    loaded_lib_implementations: HashMap<Url, Arc<dyn Implementation>>,
}

impl Loader {
    /// Create a new `Loader` that will be used to parse the manifest file create structs in
    /// memory ready for execution
    pub fn new() -> Self {
        Loader {
            loaded_libraries_manifests: HashMap::<Url, (LibraryManifest, Url)>::new(),
            loaded_lib_implementations: HashMap::<Url, Arc<dyn Implementation>>::new(),
        }
    }

    /// Return a HashMap of the Implementations loaded, with the Url for the function
    /// as the key
    pub fn get_lib_implementations(&self) -> &HashMap<Url, Arc<dyn Implementation>> {
        &self.loaded_lib_implementations
    }

    /// Load all the functions defined in a manifest, and then find all the
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
    pub fn load_flow(
        &mut self,
        server_provider: &dyn Provider,
        client_provider: &dyn Provider,
        flow_manifest_url: &Url,
    ) -> Result<FlowManifest> {
        debug!("Loading flow manifest from '{}'", flow_manifest_url);
        let (mut flow_manifest, resolved_url) =
            FlowManifest::load(server_provider, flow_manifest_url).chain_err(|| {
                format!("Error while loading manifest from: '{}'", flow_manifest_url)
            })?;

        self.load_library_implementations(server_provider, &flow_manifest)
            .chain_err(|| "Could not load library implementations for flow")?;

        // Find the implementations for all functions in this flow
        self.resolve_implementations(&mut flow_manifest, &resolved_url, client_provider)
            .chain_err(|| "Could not resolve implementations required for flow execution")?;

        Ok(flow_manifest)
    }

    /// Load the library manifest if not already loaded
    fn load_manifest_if_needed(
        &mut self,
        provider: &dyn Provider,
        lib_root_url: &Url,
    ) -> Result<()> {
        if self.loaded_libraries_manifests.get(lib_root_url).is_none() {
            info!("Attempting to load library '{}'", lib_root_url);
            let new_manifest_tuple =
                LibraryManifest::load(provider, lib_root_url).chain_err(|| {
                    format!("Could not load library with root url: '{}'", lib_root_url)
                })?;
            self.loaded_libraries_manifests
                .insert(lib_root_url.clone(), new_manifest_tuple);
        }
        Ok(())
    }

    fn get_manifest_tuple(
        &mut self,
        provider: &dyn Provider,
        lib_root_url: &Url,
    ) -> Result<(LibraryManifest, Url)> {
        self.load_manifest_if_needed(provider, lib_root_url)?;

        let tuple = self
            .loaded_libraries_manifests
            .get(lib_root_url)
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Could not find (supposedly already loaded) library manifest",
                )
            })?;

        Ok(tuple.clone())
    }

    /// Load libraries implementations referenced in the flow manifest
    fn load_library_implementations(
        &mut self,
        provider: &dyn Provider,
        manifest: &FlowManifest,
    ) -> Result<()> {
        for library_implementation_reference in manifest.get_lib_references() {
            // zero out the path to the implementation to get the root lib url
            let mut lib_root_url = library_implementation_reference.clone();
            lib_root_url.set_path("");

            let manifest_tuple = self.get_manifest_tuple(provider, &lib_root_url)?;

            self.add_lib_implementation(
                provider,
                library_implementation_reference,
                &manifest_tuple,
            )?;
        }

        Ok(())
    }

    /// Add a library and all the implementations it contains to the run-time by adding
    /// all its ImplementationLocators from the manifest to the
    /// table for this run-time, so that then when we try to load a flow that references functions
    /// in the library, they can be found.
    pub fn add_lib(
        &mut self,
        provider: &dyn Provider,
        lib_manifest: LibraryManifest,
        lib_manifest_url: &Url,
    ) -> Result<()> {
        if self
            .loaded_libraries_manifests
            .get(lib_manifest_url)
            .is_none()
        {
            let lib_manifest_tuple = (lib_manifest.clone(), lib_manifest_url.clone());

            // Load all the implementations in the library
            for (implementation_reference, locator) in lib_manifest.locators {
                // if we don't already have an implementation loaded for that reference
                if self
                    .loaded_lib_implementations
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
                    self.loaded_lib_implementations
                        .insert(implementation_reference, implementation);
                }
            }

            // track the fact we have already loaded this library
            self.loaded_libraries_manifests
                .insert(lib_manifest_url.clone(), lib_manifest_tuple);
            info!("Loaded library: '{}'", lib_manifest_url);
        }

        Ok(())
    }

    /// Add a library implementation to the table for this run-time, for use in execution
    pub fn add_lib_implementation(
        &mut self,
        provider: &dyn Provider,
        implementation_reference: &Url,
        lib_manifest_tuple: &(LibraryManifest, Url),
    ) -> Result<()> {
        // if we don't already have an implementation loaded for that reference
        if self
            .loaded_lib_implementations
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
                    debug!("Looking for wasm source: '{}'", wasm_url);
                    // Wasm implementation being added. Wrap it with the Wasm Native Implementation
                    let wasm_executor = wasm::load(provider, &wasm_url)?;
                    Arc::new(wasm_executor) as Arc<dyn Implementation>
                }
                Native(native_impl) => native_impl.clone(),
            };
            self.loaded_lib_implementations
                .insert(implementation_reference.clone(), implementation);
        }

        Ok(())
    }

    /// Resolve or "find" all the implementations of functions for a flow
    /// The `root_url` is the url of the manifest or the directory where the manifest is located
    /// and is used in resolving relative references to other files.
    pub fn resolve_implementations(
        &mut self,
        flow_manifest: &mut FlowManifest,
        manifest_url: &Url,
        provider: &dyn Provider,
    ) -> Result<()> {
        debug!("Resolving implementations");
        // find in a library, or load the supplied implementation - as specified by the source
        for function in flow_manifest.get_functions() {
            match function.implementation_location().split_once(':') {
                Some(("lib", _)) => {
                    let implementation_url = Url::parse(function.implementation_location())
                        .chain_err(|| {
                            "Could not create a Url from a lib: implementation location"
                        })?;
                    let implementation = self
                        .loaded_lib_implementations
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
}

#[cfg(test)]
mod test {
    use super::Loader;

    #[test]
    fn test_create() {
        let _ = Loader::new();
    }
}

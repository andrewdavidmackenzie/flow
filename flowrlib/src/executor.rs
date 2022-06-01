use std::collections::HashMap;
use std::panic;
use std::sync::{Arc, mpsc, Mutex};
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::time::Duration;

use log::{debug, error, info, trace};
use url::Url;

use flowcore::errors::*;
use flowcore::Implementation;
use flowcore::meta_provider::{MetaProvider, Provider};
use flowcore::model::flow_manifest::FlowManifest;
use flowcore::model::lib_manifest::{
    ImplementationLocator::Native, ImplementationLocator::Wasm, LibraryManifest,
};

use crate::job::Job;
use crate::wasm;

/// Executor structure holds information required to send jobs for execution and receive results back
/// It can load a compiled `Flow` from it's `FlowManifest`, loading the required
/// libraries needed by the flow and keeping track of the `Function` `Implementations` that
/// will be used to execute it.
pub struct Executor {
    /// A channel used to send Jobs out for execution
    job_tx: Sender<Job>,
    /// A channel used to receive Jobs back after execution (now including the job's output)
    job_rx: Receiver<Job>,
    /// The timeout for waiting for results back from jobs being executed
    job_timeout: Option<Duration>,
    /// HashMap of libraries already loaded. The key is the library reference Url
    /// (e.g. lib:://flowstdlib) and the entry is a tuple of the LibraryManifest
    /// and the resolved Url of where the manifest was read from
    loaded_libraries: HashMap<Url, (LibraryManifest, Url)>,
    loaded_implementations: HashMap<Url, Arc<dyn Implementation>>,
    /// A MetaProvider used to fetch content for implementation and manifest loading etc
    provider: MetaProvider,
}

/// Struct that takes care of execution of jobs, sending jobs for execution and receiving results
impl Executor {
    /// Create a new `Executor` specifying the number of local executor threads and a timeout
    /// for reception of results back to avoid blocking
    pub fn new(provider: MetaProvider, number_of_executors: usize, job_timeout: Option<Duration>) -> Self {
        let (job_tx, job_rx) = mpsc::channel();
        let (output_tx, output_rx) = mpsc::channel();

        info!("Starting {} local executor threads", number_of_executors);
        let shared_job_receiver = Arc::new(Mutex::new(job_rx));
        start_executors(number_of_executors, &shared_job_receiver, &output_tx);

        Executor {
            job_tx,
            job_rx: output_rx,
            job_timeout,
            loaded_libraries: HashMap::<Url, (LibraryManifest, Url)>::new(),
            loaded_implementations: HashMap::<Url, Arc<dyn Implementation>>::new(),
            provider
        }
    }

    /// Set the timeout to use when waiting for job results after execution
    pub fn set_timeout(&mut self, timeout: Option<Duration>) {
        self.job_timeout = timeout;
    }

    /// Wait for, then return the next Job with results returned from executors
    pub fn get_next_result(&mut self) -> Result<Job> {
        match self.job_timeout {
            Some(t) => self.job_rx.recv_timeout(t)
                .chain_err(|| "Timeout while waiting for Job result"),
            None => self.job_rx.recv()
                .chain_err(|| "Error while trying to receive Job results")
        }
    }

    /// Send a `Job` for execution to executors
    pub fn send_job_for_execution(&mut self, job: &Job) -> Result<()> {
        self.job_tx
            .send(job.clone())
            .chain_err(|| "Sending of job for execution failed")?;

        trace!(
            "Job #{}: Sent for Execution of Function #{}",
            job.job_id,
            job.function_id
        );

        Ok(())
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
        flow_manifest_url: &Url,
    ) -> Result<FlowManifest> {
        debug!("Loading flow manifest from '{}'", flow_manifest_url);
        let (mut flow_manifest, resolved_url) =
            FlowManifest::load(&self.provider as &dyn Provider, flow_manifest_url)
                .chain_err(|| format!("Could not load manifest from: '{}'", flow_manifest_url))?;

        self.load_lib_implementations(&flow_manifest)
            .chain_err(|| format!("Could not load libraries referenced by manifest at: {}",
                                  resolved_url))?;

        // Find the implementations for all functions used in this flow
        self.resolve_function_implementations(&mut flow_manifest, &resolved_url)
            .chain_err(|| "Could not resolve implementations required for flow execution")?;

        Ok(flow_manifest)
    }

    // Load the library manifest if is not already loaded
    fn load_lib_manifest_if_needed(
        &mut self,
        lib_root_url: &Url,
    ) -> Result<()> {
        if self.loaded_libraries.get(lib_root_url).is_none() {
            info!("Attempting to load library '{}'", lib_root_url);
            let new_manifest_tuple =
                LibraryManifest::load(&self.provider as &dyn Provider, lib_root_url).chain_err(|| {
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
        lib_root_url: &Url,
    ) -> Result<(LibraryManifest, Url)> {
        self.load_lib_manifest_if_needed(lib_root_url)?;

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
        flow_manifest: &FlowManifest,
    ) -> Result<()> {
        for lib_reference in flow_manifest.get_lib_references() {
            // zero out the path to the implementation to get the root lib url
            let mut lib_root_url = lib_reference.clone();
            lib_root_url.set_path("");

            let manifest_tuple = self.get_lib_manifest_tuple(&lib_root_url)?;

            self.load_lib_implementation(
                lib_reference,
                &manifest_tuple,
            )?;
        }

        Ok(())
    }

    /// Load a library and all the implementations it contains into the loader.
    /// They are references by Url so they can be found when loading a flow that requires them.
    pub fn load_lib(
        &mut self,
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
                let implementation = match locator {
                    Wasm(wasm_source_relative) => {
                        // Path to the wasm source could be relative to the URL where we loaded the manifest from
                        let wasm_url = lib_manifest_url
                            .join(&wasm_source_relative)
                            .map_err(|e| e.to_string())?;
                        debug!("Attempting to load wasm from: '{}'", wasm_url);
                        // Wasm loaded, wrap it with the Wasm Native Implementation
                        Arc::new(wasm::load(&self.provider as &dyn Provider, &wasm_url)?) as Arc<dyn Implementation>
                    }

                    // Native implementation from Lib
                    Native(implementation) => implementation,
                };
                self.loaded_implementations
                    .insert(implementation_reference, implementation);
            }

            // track the fact we have loaded this library manifest
            self.loaded_libraries
                .insert(lib_manifest_url.clone(), lib_manifest_tuple);
            info!("Loaded '{}'", lib_manifest_url);
        }

        Ok(())
    }

    // Add a library implementation to the loader
    fn load_lib_implementation(
        &mut self,
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
                    let wasm_executor = wasm::load(&self.provider as &dyn Provider, &wasm_url)?;
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
        flow_manifest: &mut FlowManifest,
        manifest_url: &Url,
    ) -> Result<()> {
        debug!("Resolving implementations");
        // find in a library, or load the supplied implementation - as specified by the source
        for function in flow_manifest.get_functions() {
            let implementation =
                self.resolve_implementation(manifest_url,function.implementation_location())?;
            function.set_implementation(implementation);
        }

        Ok(())
    }

    fn resolve_implementation(&mut self,
                              manifest_url: &Url,
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
                let wasm_executor = wasm::load(&self.provider as &dyn Provider, &implementation_url)?;
                Ok(Arc::new(wasm_executor) as Arc<dyn Implementation>)
            }
        }
    }
}

// Start a number of executor threads that all listen on the 'job_rx' channel for
// Jobs to execute and return the Outputs on the 'output_tx' channel
fn start_executors(
    number_of_executors: usize,
    job_rx: &Arc<Mutex<Receiver<Job>>>,
    job_tx: &Sender<Job>,
) {
    for executor_number in 0..number_of_executors {
        create_executor(
            format!("Executor #{}", executor_number),
            job_rx.clone(),
            job_tx.clone(),
        ); // clone of Arcs and Sender OK
    }
}

fn create_executor(name: String, job_rx: Arc<Mutex<Receiver<Job>>>, job_tx: Sender<Job>) {
    let builder = thread::Builder::new();
    let _ = builder.spawn(move || {
        set_panic_hook();

        loop {
            let _ = get_and_execute_job(&job_rx, &job_tx, &name);
        }
    });
}

// Replace the standard panic hook with one that just outputs the file and line of any process's
// run-time panic.
fn set_panic_hook() {
    panic::set_hook(Box::new(|panic_info| {
        /* Only available on 'nightly'
        if let Some(message) = panic_info.message() {
            error!("Message: {:?}", message);
        }
        */

        if let Some(location) = panic_info.location() {
            error!(
                "Panic in file '{}' at line {}",
                location.file(),
                location.line()
            );
        }
    }));
}

fn get_and_execute_job(
    job_rx: &Arc<Mutex<Receiver<Job>>>,
    job_tx: &Sender<Job>,
    name: &str,
) -> Result<()> {
    let guard = job_rx
        .lock()
        .map_err(|e| format!("Error locking receiver to get job: '{}'", e))?;
    let job = guard
        .recv()
        .map_err(|e| format!("Error receiving job for execution: '{}'", e))?;
    execute(job, job_tx, name)
}

fn execute(mut job: Job, job_tx: &Sender<Job>, name: &str) -> Result<()> {
    trace!("Job #{}: Started  executing on '{name}'", job.job_id);
    job.result = job.implementation.run(&job.input_set);
    trace!("Job #{}: Finished executing on '{name}'", job.job_id);
    job_tx
        .send(job)
        .chain_err(|| "Error sending job result back after execution")
}

#[cfg(test)]
mod test {
    use std::fs::File;
    use std::io::{self, Read};
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    use serde_json::Value;
    use simpath::Simpath;
    use tempdir::TempDir;
    use url::Url;

    use flowcore::{DONT_RUN_AGAIN, Implementation, RunAgain};
    use flowcore::errors::Result;
    use flowcore::meta_provider::MetaProvider;
    use flowcore::model::flow_manifest::FlowManifest;
    use flowcore::model::lib_manifest::{ImplementationLocator::Native, LibraryManifest};
    use flowcore::model::metadata::MetaData;
    use flowcore::model::runtime_function::RuntimeFunction;

    use crate::executor::Executor;

    fn create_test_flow_manifest(functions: Vec<RuntimeFunction>) -> FlowManifest {
        let metadata = MetaData {
            name: "test manifest".into(),
            description: "test manifest".into(),
            version: "0.0".into(),
            authors: vec!["me".into()],
        };

        let mut manifest = FlowManifest::new(metadata);

        for function in functions {
            let location_url = &Url::parse(function.implementation_location())
                .expect("Could not create Url");
            match location_url.scheme() {
                "lib" => manifest.add_lib_reference(location_url),
                "context" => manifest.add_context_reference(location_url),
                _ => {}
            }
            manifest.add_function(function);
        }

        manifest
    }

    #[derive(Debug)]
    struct Fake;

    impl Implementation for Fake {
        fn run(&self, mut _inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
            let mut value = None;

            let mut buffer = String::new();
            if let Ok(size) = io::stdin().lock().read_to_string(&mut buffer) {
                if size > 0 {
                    let input = Value::String(buffer.trim().to_string());
                    value = Some(input);
                }
            }

            Ok((value, DONT_RUN_AGAIN))
        }
    }

    fn create_test_context_manifest() -> LibraryManifest {
        let metadata = MetaData {
            name: "context".to_string(),
            description: "".into(),
            version: "0.1.0".into(),
            authors: vec!["".into()],
        };
        let lib_url = Url::parse("context://").expect("Couldn't create lib url");
        let mut manifest = LibraryManifest::new(lib_url, metadata);

        manifest.locators.insert(
            Url::parse("context://args/get/get").expect("Could not create Url"),
            Native(Arc::new(Fake {})),
        );
        manifest.locators.insert(
            Url::parse("context://file/file_write/file_write").expect("Could not create Url"),
            Native(Arc::new(Fake {})),
        );
        manifest.locators.insert(
            Url::parse("context://stdio/readline/readline").expect("Could not create Url"),
            Native(Arc::new(Fake {})),
        );
        manifest.locators.insert(
            Url::parse("context://stdio/stdin/stdin").expect("Could not create Url"),
            Native(Arc::new(Fake {})),
        );
        manifest.locators.insert(
            Url::parse("context://stdio/stdout/stdout").expect("Could not create Url"),
            Native(Arc::new(Fake {})),
        );
        manifest.locators.insert(
            Url::parse("context://stdio/stderr/stderr").expect("Could not create Url"),
            Native(Arc::new(Fake {})),
        );

        manifest
    }

    fn write_manifest(manifest: &FlowManifest, filename: &Path) -> Result<()> {
        let mut manifest_file =
            File::create(&filename).map_err(|_| "Could not create lib manifest file")?;

        manifest_file
            .write_all(
                serde_json::to_string_pretty(manifest)
                    .map_err(|_| "Could not pretty format the manifest JSON contents")?
                    .as_bytes(),
            )
            .map_err(|_| "Could not write manifest data bytes to created manifest file")?;

        Ok(())
    }

    #[test]
    fn load_manifest_from_file() {
        let f_a = RuntimeFunction::new(
            #[cfg(feature = "debugger")] "fA",
            #[cfg(feature = "debugger")] "/fA",
            "context://stdio/stdout/stdout",
            vec![],
            0,
            0,
            &[],
            false,
        );
        let functions = vec![f_a];
        let manifest = create_test_flow_manifest(functions);

        let temp_dir = TempDir::new("flow").expect("Could not get temp dir").into_path();
        let manifest_file = temp_dir.join("manifest.json");
        write_manifest(&manifest, &manifest_file).expect("Could not write manifest file");
        let manifest_url = Url::from_directory_path(manifest_file).expect("Could not create url from directory path");
        let provider = MetaProvider::new(Simpath::new("FLOW_LIB_PATH"),
                                         PathBuf::from("/"));

        let mut executor = Executor::new(provider, 0, None);
        executor
            .load_lib(
                create_test_context_manifest(),
                &Url::parse("context://").expect("Could not parse lib url"),
            )
            .expect("Could not add context library to loader");

        assert!(executor.load_flow(&manifest_url).is_ok());
    }

    #[test]
    fn unresolved_references_test() {
        let f_a = RuntimeFunction::new(
            #[cfg(feature = "debugger")] "fA",
            #[cfg(feature = "debugger")] "/fA",
            "context://stdio/stdout/foo",
            vec![],
            0,
            0,
            &[],
            false,
        );
        let functions = vec![f_a];
        let manifest = create_test_flow_manifest(functions);

        let temp_dir = TempDir::new("flow").expect("Could not get temp dir").into_path();
        let manifest_file = temp_dir.join("manifest.json");
        write_manifest(&manifest, &manifest_file).expect("Could not write manifest file");
        let manifest_url = Url::from_directory_path(manifest_file).expect("Could not create url from directory path");
        let provider = MetaProvider::new(Simpath::new("FLOW_LIB_PATH"),
                                         PathBuf::from("/"));

        let mut executor = Executor::new(provider, 0, None);
        executor
            .load_lib(
                create_test_context_manifest(),
                &Url::parse("context://").expect("Could not parse lib url"),
            )
            .expect("Could not add context library to loader");

        assert!(executor
            .load_flow(&manifest_url).is_err());
    }
}

use std::collections::HashMap;
use std::panic;
use std::sync::{Arc, RwLock};
use std::thread;
use std::thread::JoinHandle;

use log::{debug, error, info, trace};
use url::Url;

use flowcore::errors::{bail, Result, ResultExt};
use flowcore::model::lib_manifest::{
    ImplementationLocator::Native, ImplementationLocator::RelativePath, LibraryManifest,
};
use flowcore::provider::Provider;
use flowcore::Implementation;

use crate::job::Payload;
use crate::wasm;

/// An `Executor` struct is used to receive jobs, execute them, and return results.
/// It can load libraries and keep track of the `Function` `Implementations` loaded for use
/// in job execution.
pub struct Executor {
    // HashMap of library manifests already loaded. The key is the library reference Url
    // (e.g. lib:://flowstdlib), and the entry is a tuple of the LibraryManifest
    // and the resolved Url of where the manifest was read from
    loaded_lib_manifests: Arc<RwLock<HashMap<Url, (LibraryManifest, Url)>>>,
    executors: Vec<JoinHandle<()>>,
}

impl Default for Executor {
    fn default() -> Self {
        Self::new()
    }
}

impl Executor {
    /// Create a new executor that receives jobs, executes them, and returns results.
    #[must_use]
    pub fn new() -> Self {
        Executor {
            loaded_lib_manifests: Arc::new(RwLock::new(
                HashMap::<Url, (LibraryManifest, Url)>::new(),
            )),
            executors: vec![],
        }
    }

    /// Add a library manifest so that it can be used later on to load implementations that are
    /// required to execute jobs. Also provide the Url that the library url resolves to, so that
    /// later it can be used when resolving the locations of implementations in this library.
    ///
    /// # Errors
    ///
    /// Returns an error if this `LibraryManifest` cannot be added to the set of manifests used
    /// by the runtime to load functions.
    ///
    pub fn add_lib(&mut self, lib_manifest: LibraryManifest, resolved_url: Url) -> Result<()> {
        let mut lib_manifests = self
            .loaded_lib_manifests
            .write()
            .map_err(|_| "Could not gain write access to loaded library manifests map")?;

        debug!(
            "Manifest of library '{}' loaded from '{}' and added to Executor",
            lib_manifest.lib_url, resolved_url
        );

        lib_manifests.insert(lib_manifest.lib_url.clone(), (lib_manifest, resolved_url));

        Ok(())
    }

    /// Start executing jobs, specifying:
    /// - the `Provider` to use to fetch implementation content
    /// - the number of executor threads
    /// - the address of the job socket to get jobs from
    /// - the address of the results socket to return results from executed jobs to
    pub fn start(
        &mut self,
        provider: &Arc<dyn Provider>,
        number_of_executors: usize,
        job_service: &str,
        results_service: &str,
        control_service: &str,
    ) {
        self.start_with_options(
            provider,
            number_of_executors,
            job_service,
            results_service,
            control_service,
            false,
        );
    }

    /// Like [`start`](Self::start) but with `spawn_jobs = true`, each job is
    /// executed on a spawned thread so one blocking job doesn't prevent the
    /// executor thread from picking up the next job. This is used by the context
    /// executor where blocking IO (readline/stdin) must not prevent non-blocking
    /// IO (stdout/stderr) from executing concurrently.
    pub fn start_spawning(
        &mut self,
        provider: &Arc<dyn Provider>,
        number_of_executors: usize,
        job_service: &str,
        results_service: &str,
        control_service: &str,
    ) {
        self.start_with_options(
            provider,
            number_of_executors,
            job_service,
            results_service,
            control_service,
            true,
        );
    }

    fn start_with_options(
        &mut self,
        provider: &Arc<dyn Provider>,
        number_of_executors: usize,
        job_service: &str,
        results_service: &str,
        control_service: &str,
        spawn_jobs: bool,
    ) {
        let loaded_implementations =
            Arc::new(RwLock::new(HashMap::<Url, Arc<dyn Implementation>>::new()));

        info!("Starting {number_of_executors} executor threads");
        for executor_number in 0..number_of_executors {
            let thread_provider = provider.clone();
            let thread_context = zmq::Context::new();
            let thread_implementations = loaded_implementations.clone();
            let thread_loaded_manifests = self.loaded_lib_manifests.clone();
            let results_sink = results_service.into();
            let job_source = job_service.into();
            let control_address = control_service.into();
            self.executors.push(thread::spawn(move || {
                trace!("Executor #{executor_number} entering execution loop");
                if let Err(e) = execution_loop(
                    &thread_provider,
                    &format!("Executor #{executor_number}"),
                    &thread_context,
                    &thread_implementations,
                    &thread_loaded_manifests,
                    job_source,
                    results_sink,
                    control_address,
                    spawn_jobs,
                ) {
                    error!("Execution loop error: {e}");
                }
            }));
        }
    }

    /// Wait until all threads end
    pub fn wait(self) {
        for executor in self.executors {
            let _ = executor.join();
        }
    }
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
#[allow(clippy::needless_pass_by_value)]
fn execution_loop(
    provider: &Arc<dyn Provider>,
    name: &str,
    context: &zmq::Context,
    loaded_implementations: &Arc<RwLock<HashMap<Url, Arc<dyn Implementation>>>>,
    loaded_lib_manifests: &Arc<RwLock<HashMap<Url, (LibraryManifest, Url)>>>,
    job_service: String,
    results_service: String,
    control_address: String,
    spawn_jobs: bool,
) -> Result<()> {
    let job_source = context
        .socket(zmq::PULL)
        .map_err(|e| format!("Could not create PULL end of job socket: {e}"))?;
    job_source
        .connect(&job_service)
        .map_err(|e| format!("Could not connect to PULL end of job socket: '{job_service}' {e}"))?;

    let results_sink = context
        .socket(zmq::PUSH)
        .map_err(|e| format!("Could not create PUSH end of results socket: {e}"))?;
    results_sink
        .connect(&results_service)
        .map_err(|e| format!("Could not connect to PUSH end of results socket: {e}"))?;

    let control_socket = context
        .socket(zmq::SocketType::SUB)
        .map_err(|e| format!("Could not create SUB end of control socket: {e}"))?;
    control_socket
        .connect(&control_address)
        .map_err(|e| format!("Could not connect to SUB end of control socket: {e}"))?;
    control_socket
        .set_subscribe(&[])
        .map_err(|e| format!("Could not subscribe to SUB end of control socket: {e}"))?;

    let mut process_jobs = true;

    set_panic_hook();

    // When spawning jobs, use a channel to relay results from spawned threads
    // back to this thread (which owns the ZMQ results socket).
    let spawn_result_tx = if spawn_jobs {
        let (tx, rx) = std::sync::mpsc::channel::<String>();
        // Leak the receiver into a static — it will be polled in the loop below.
        // This is safe because the execution_loop runs for the lifetime of the thread.
        Some((tx, rx))
    } else {
        None
    };

    let mut items: Vec<zmq::PollItem> = vec![
        job_source.as_poll_item(zmq::POLLIN),
        control_socket.as_poll_item(zmq::POLLIN),
    ];

    while process_jobs {
        // When spawning jobs, drain results from spawned threads first
        if let Some((_, ref rx)) = spawn_result_tx {
            while let Ok(serialized_result) = rx.try_recv() {
                results_sink
                    .send(serialized_result.as_bytes(), 0)
                    .map_err(|_| "Could not send result of Job from spawned thread")?;
            }
        }

        trace!("{name} waiting for a job to execute or a DONE signal");
        let poll_timeout = if spawn_jobs { 100 } else { -1 };
        match zmq::poll(&mut items, poll_timeout)
            .map_err(|_| "Error while polling for Jobs to execute")
        {
            Ok(_) => {
                if items
                    .first()
                    .ok_or("Could not get poll item 0")?
                    .is_readable()
                {
                    let msg = job_source
                        .recv_msg(0)
                        .map_err(|_| "Error receiving Job for execution")?;
                    let message_string = msg.as_str().ok_or("Could not get message as str")?;
                    let payload: Payload = serde_json::from_str(message_string)
                        .map_err(|_| "Could not deserialize Message to Job")?;

                    trace!("Job #{}: Received by {}", payload.job_id, name);

                    let is_blocking_io = spawn_jobs
                        && (payload.implementation_url.as_str() == "context://stdio/readline"
                            || payload.implementation_url.as_str() == "context://stdio/stdin");
                    if is_blocking_io {
                        // Spawn blocking IO jobs on a separate thread so this
                        // executor can pick up the next job immediately.
                        // Only readline and stdin are spawned — all other jobs
                        // run synchronously to preserve output ordering.
                        let sp = provider.clone();
                        let sn = name.to_string();
                        let si = loaded_implementations.clone();
                        let sm = loaded_lib_manifests.clone();
                        let stx = spawn_result_tx
                            .as_ref()
                            .map(|(tx, _)| tx.clone())
                            .ok_or("spawn_result_tx missing")?;
                        thread::spawn(move || {
                            match execute_job_to_string(&sp, &payload, &sn, &si, &sm) {
                                Ok(serialized) => {
                                    let _ = stx.send(serialized);
                                }
                                Err(e) => error!("{e}"),
                            }
                        });
                    } else {
                        match execute_job(
                            provider,
                            &payload,
                            &results_sink,
                            name,
                            &loaded_implementations.clone(),
                            &loaded_lib_manifests.clone(),
                        ) {
                            Ok(keep_processing) => process_jobs = keep_processing,
                            Err(e) => error!("{e}"),
                        }
                    }
                }

                if items
                    .get(1)
                    .ok_or("Could not get poll item 1")?
                    .is_readable()
                {
                    let msg = control_socket
                        .recv_msg(0)
                        .map_err(|_| "Error receiving Control message")?;
                    match msg.as_str().ok_or("Could not get message as str") {
                        Ok("DONE") => {
                            trace!("'DONE' message received in executor");
                            return Ok(());
                        }
                        Ok(_) => error!("Unexpected Control message"),
                        _ => error!("Error parsing Control message"),
                    }
                }
            }
            Err(e) => {
                error!("Error while polling for Jobs or Control messages: {e}");
            }
        }
    }

    Ok(())
}

/// Execute a job and return the serialized result string (for spawned execution).
/// Like `execute_job` but does not send the result over ZMQ directly.
fn execute_job_to_string(
    provider: &Arc<dyn Provider>,
    payload: &Payload,
    name: &str,
    loaded_implementations: &Arc<RwLock<HashMap<Url, Arc<dyn Implementation>>>>,
    loaded_lib_manifests: &Arc<RwLock<HashMap<Url, (LibraryManifest, Url)>>>,
) -> Result<String> {
    let implementation = get_or_load_implementation(
        provider,
        payload,
        loaded_implementations,
        loaded_lib_manifests,
    )?;

    trace!("Job #{}: Started executing on '{name}'", payload.job_id);
    let result = implementation.run(&payload.input_set);
    trace!("Job #{}: Finished executing on '{name}'", payload.job_id);

    serde_json::to_string(&(payload.job_id, result))
        .map_err(|e| format!("Could not serialize job result: {e}").into())
}

// Replace the standard panic hook with one that just outputs the file and line of any panic.
fn set_panic_hook() {
    panic::set_hook(Box::new(|panic_info| {
        if let Some(location) = panic_info.location() {
            error!(
                "Panic in file '{}' at line {}",
                location.file(),
                location.line()
            );
        }
    }));
}

/// Get (or load) the implementation for a job, releasing the lock before returning.
/// This is critical: `run()` may block (e.g. readline waiting for user input) and
/// holding the lock would prevent other executor threads from running concurrently.
fn get_or_load_implementation(
    provider: &Arc<dyn Provider>,
    payload: &Payload,
    loaded_implementations: &Arc<RwLock<HashMap<Url, Arc<dyn Implementation>>>>,
    loaded_lib_manifests: &Arc<RwLock<HashMap<Url, (LibraryManifest, Url)>>>,
) -> Result<Arc<dyn Implementation>> {
    // First try a read lock to avoid contention
    let needs_load = {
        let implementations = loaded_implementations
            .read()
            .map_err(|_| "Could not gain read access to loaded implementations map")?;
        implementations.get(&payload.implementation_url).is_none()
    };

    if needs_load {
        trace!(
            "Implementation '{}' is not loaded",
            payload.implementation_url
        );
        let mut implementations = loaded_implementations
            .write()
            .map_err(|_| "Could not gain write access to loaded implementations map")?;
        // Double-check after acquiring write lock (another thread may have loaded it)
        if implementations.get(&payload.implementation_url).is_none() {
            let impl_arc = match payload.implementation_url.scheme() {
                "lib" => {
                    let mut lib_root_url = payload.implementation_url.clone();
                    lib_root_url.set_path("");
                    load_referenced_implementation(
                        provider,
                        &lib_root_url,
                        loaded_lib_manifests,
                        &payload.implementation_url,
                    )?
                }
                "context" => {
                    let mut lib_root_url = payload.implementation_url.clone();
                    let _ = lib_root_url.set_host(Some(""));
                    lib_root_url.set_path("");
                    load_referenced_implementation(
                        provider,
                        &lib_root_url,
                        loaded_lib_manifests,
                        &payload.implementation_url,
                    )?
                }
                "file" => Arc::new(wasm::load(provider, &payload.implementation_url)?),
                _ => bail!("Unsupported scheme on implementation_url"),
            };
            implementations.insert(payload.implementation_url.clone(), impl_arc);
            trace!(
                "Implementation '{}' added to executor",
                payload.implementation_url
            );
        }
        Ok(Arc::clone(
            implementations
                .get(&payload.implementation_url)
                .ok_or("Could not find implementation")?,
        ))
    } else {
        let implementations = loaded_implementations
            .read()
            .map_err(|_| "Could not gain read access to loaded implementations map")?;
        Ok(Arc::clone(
            implementations
                .get(&payload.implementation_url)
                .ok_or("Could not find implementation")?,
        ))
    }
}

// Return Ok(keep_processing) flag as true or false to keep processing
fn execute_job(
    provider: &Arc<dyn Provider>,
    payload: &Payload,
    results_sink: &zmq::Socket,
    name: &str,
    loaded_implementations: &Arc<RwLock<HashMap<Url, Arc<dyn Implementation>>>>,
    loaded_lib_manifests: &Arc<RwLock<HashMap<Url, (LibraryManifest, Url)>>>,
) -> Result<bool> {
    let implementation = get_or_load_implementation(
        provider,
        payload,
        loaded_implementations,
        loaded_lib_manifests,
    )?;

    trace!("Job #{}: Started executing on '{name}'", payload.job_id);
    let result = implementation.run(&payload.input_set);
    trace!("Job #{}: Finished executing on '{name}'", payload.job_id);

    results_sink
        .send(
            serde_json::to_string(&(payload.job_id, result))?.as_bytes(),
            0,
        )
        .map_err(|_| "Could not send result of Job")?;

    Ok(true)
}

// Load a context or library implementation
fn load_referenced_implementation(
    provider: &Arc<dyn Provider>,
    lib_root_url: &Url,
    loaded_lib_manifests: &Arc<RwLock<HashMap<Url, (LibraryManifest, Url)>>>,
    implementation_url: &Url,
) -> Result<Arc<dyn Implementation>> {
    let (lib_manifest, resolved_lib_url) =
        get_lib_manifest_tuple(provider, loaded_lib_manifests, lib_root_url)?;

    let locator = lib_manifest
        .locators
        .get(implementation_url)
        .ok_or(format!(
            "Could not find implementation for '{implementation_url}' in library '{}'.\n\
             The library may need to be recompiled or reinstalled.\n\
             For flowstdlib: curl -sL https://github.com/andrewdavidmackenzie/flow/releases/download/v{}/install-flowstdlib.sh | bash",
            lib_manifest.metadata.name,
            env!("CARGO_PKG_VERSION")
        ))?;

    // find the implementation we need from the locator
    let implementation = match locator {
        RelativePath(wasm_source_relative) => {
            // Path to the wasm source could be relative to the URL where we loaded the manifest from
            let wasm_url = resolved_lib_url
                .join(wasm_source_relative)
                .map_err(|e| e.to_string())?;
            debug!("Attempting to load wasm from source file: '{wasm_url}'");
            // Wasm implementation being added. Wrap it with the Wasm Native Implementation
            let wasm_executor = wasm::load(provider, &wasm_url)?;
            Arc::new(wasm_executor) as Arc<dyn Implementation>
        }
        Native(native_impl) => native_impl.clone(),
    };

    Ok(implementation)
}

// Get the tuple of the lib manifest and the url from where it was loaded from
fn get_lib_manifest_tuple(
    provider: &Arc<dyn Provider>,
    loaded_lib_manifests: &Arc<RwLock<HashMap<Url, (LibraryManifest, Url)>>>,
    lib_root_url: &Url,
) -> Result<(LibraryManifest, Url)> {
    let mut lib_manifests = loaded_lib_manifests
        .write()
        .map_err(|_| "Could not get write access to the loaded lib manifests")?;

    if lib_manifests.get(lib_root_url).is_none() {
        info!("Attempting to load library manifest'{lib_root_url}'");
        let manifest_tuple = LibraryManifest::load(provider, lib_root_url)
            .chain_err(|| format!("Could not load library with root url: '{lib_root_url}'"))?;
        lib_manifests.insert(lib_root_url.clone(), manifest_tuple);
    }

    // TODO avoid this clone and return references
    lib_manifests
        .get(lib_root_url)
        .ok_or_else(|| "Could not find (supposedly already loaded) library manifest".into())
        .cloned()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {
    use std::collections::HashMap;
    use std::sync::{Arc, RwLock};

    use portpicker::pick_unused_port;
    use serial_test::serial;
    use url::Url;

    use flowcore::errors::Result;
    use flowcore::model::lib_manifest::LibraryManifest;
    use flowcore::model::metadata::MetaData;
    use flowcore::provider::Provider;
    use flowcore::Implementation;

    use crate::job::{Job, Payload};

    use super::Executor;

    fn test_meta_data() -> MetaData {
        MetaData {
            name: "test".into(),
            version: "0.0.0".into(),
            description: "a test".into(),
            authors: vec!["me".into()],
        }
    }

    #[allow(clippy::module_name_repetitions)]
    pub struct TestProvider {
        test_content: &'static str,
    }

    impl Provider for TestProvider {
        fn resolve_url(
            &self,
            source: &Url,
            _default_filename: &str,
            _extensions: &[&str],
        ) -> Result<(Url, Option<Url>)> {
            Ok((source.clone(), None))
        }

        fn get_contents(&self, _url: &Url) -> Result<Vec<u8>> {
            Ok(self.test_content.as_bytes().to_owned())
        }
    }

    /// A test provider that resolves URLs to a `.json` file URL, allowing
    /// `LibraryManifest::load` to find the correct JSON deserializer.
    struct ManifestTestProvider {
        test_content: &'static str,
    }

    impl Provider for ManifestTestProvider {
        fn resolve_url(
            &self,
            _source: &Url,
            _default_filename: &str,
            _extensions: &[&str],
        ) -> Result<(Url, Option<Url>)> {
            // Return a file:// URL with .json extension so the deserializer can be found
            let resolved = Url::parse("file:///tmp/manifest.json").map_err(|e| e.to_string())?;
            Ok((resolved, None))
        }

        fn get_contents(&self, _url: &Url) -> Result<Vec<u8>> {
            Ok(self.test_content.as_bytes().to_owned())
        }
    }

    #[test]
    fn add_a_lib() {
        let library = LibraryManifest::new(
            Url::parse("lib://testlib").expect("Could not parse lib url"),
            test_meta_data(),
        );

        let mut executor = Executor::new();
        assert!(executor
            .add_lib(
                library,
                Url::parse("file://fake/lib/location").expect("Could not parse Url")
            )
            .is_ok());
    }

    #[test]
    fn default_same_as_new() {
        let default_executor = Executor::default();
        let new_executor = Executor::new();

        // Both should start with empty lib manifests
        let default_manifests = default_executor
            .loaded_lib_manifests
            .read()
            .expect("Could not read default executor manifests");
        let new_manifests = new_executor
            .loaded_lib_manifests
            .read()
            .expect("Could not read new executor manifests");

        assert!(
            default_manifests.is_empty(),
            "default() executor should have no loaded manifests"
        );
        assert!(
            new_manifests.is_empty(),
            "new() executor should have no loaded manifests"
        );

        // Both should start with no executor threads
        assert!(
            default_executor.executors.is_empty(),
            "default() executor should have no executor threads"
        );
        assert!(
            new_executor.executors.is_empty(),
            "new() executor should have no executor threads"
        );
    }

    #[test]
    fn add_multiple_libs() {
        let lib1 = LibraryManifest::new(
            Url::parse("lib://testlib1").expect("Could not parse lib url"),
            test_meta_data(),
        );
        let lib2 = LibraryManifest::new(
            Url::parse("lib://testlib2").expect("Could not parse lib url"),
            MetaData {
                name: "test2".into(),
                version: "0.0.1".into(),
                description: "another test".into(),
                authors: vec!["someone".into()],
            },
        );

        let mut executor = Executor::new();
        assert!(executor
            .add_lib(
                lib1,
                Url::parse("file://fake/lib1/location").expect("Could not parse Url")
            )
            .is_ok());
        assert!(executor
            .add_lib(
                lib2,
                Url::parse("file://fake/lib2/location").expect("Could not parse Url")
            )
            .is_ok());

        let manifests = executor
            .loaded_lib_manifests
            .read()
            .expect("Could not read manifests");
        assert_eq!(manifests.len(), 2, "Should have two loaded manifests");
        assert!(
            manifests.contains_key(&Url::parse("lib://testlib1").expect("Could not parse lib url"))
        );
        assert!(
            manifests.contains_key(&Url::parse("lib://testlib2").expect("Could not parse lib url"))
        );
    }

    #[test]
    fn add_same_lib_twice_overwrites() {
        let lib_url = Url::parse("lib://testlib").expect("Could not parse lib url");

        let lib1 = LibraryManifest::new(
            lib_url.clone(),
            MetaData {
                name: "original".into(),
                version: "0.0.0".into(),
                description: "original lib".into(),
                authors: vec!["me".into()],
            },
        );
        let lib2 = LibraryManifest::new(
            lib_url.clone(),
            MetaData {
                name: "replacement".into(),
                version: "1.0.0".into(),
                description: "replacement lib".into(),
                authors: vec!["someone_else".into()],
            },
        );

        let resolved1 = Url::parse("file://fake/lib/location1").expect("Could not parse Url");
        let resolved2 = Url::parse("file://fake/lib/location2").expect("Could not parse Url");

        let mut executor = Executor::new();
        assert!(executor.add_lib(lib1, resolved1).is_ok());
        assert!(executor.add_lib(lib2, resolved2.clone()).is_ok());

        let manifests = executor
            .loaded_lib_manifests
            .read()
            .expect("Could not read manifests");
        assert_eq!(
            manifests.len(),
            1,
            "Should have only one manifest after adding the same lib_url twice"
        );

        let (manifest, resolved_url) = manifests
            .get(&lib_url)
            .expect("Could not find manifest for lib url");
        assert_eq!(
            manifest.metadata.name, "replacement",
            "Manifest should be the second (replacement) one"
        );
        assert_eq!(
            manifest.metadata.version, "1.0.0",
            "Version should be from the replacement manifest"
        );
        assert_eq!(
            *resolved_url, resolved2,
            "Resolved URL should be from the second add_lib call"
        );
    }

    #[test]
    fn execute_job_unsupported_scheme() {
        let payload = Payload {
            job_id: 0,
            input_set: vec![],
            implementation_url: Url::parse("http://example.com/some/impl")
                .expect("Could not parse Url"),
        };

        let loaded_implementations =
            Arc::new(RwLock::new(HashMap::<Url, Arc<dyn Implementation>>::new()));
        let loaded_lib_manifests =
            Arc::new(RwLock::new(HashMap::<Url, (LibraryManifest, Url)>::new()));
        let provider = Arc::new(TestProvider { test_content: "" }) as Arc<dyn Provider>;
        let context = zmq::Context::new();
        let results_sink = context
            .socket(zmq::PUSH)
            .expect("Could not create PUSH end of results-sink socket");
        results_sink
            .connect("tcp://127.0.0.1:3459")
            .expect("Could not connect to PUSH end of results-sink socket");

        let result = super::execute_job(
            &provider,
            &payload,
            &results_sink,
            "test executor",
            &loaded_implementations,
            &loaded_lib_manifests,
        );

        assert!(result.is_err(), "Unsupported scheme should return an error");
        let err_msg = result.expect_err("Expected an error").to_string();
        assert!(
            err_msg.contains("Unsupported scheme"),
            "Error should mention unsupported scheme, got: {err_msg}"
        );
    }

    #[test]
    fn execute_job_lib_impl_not_in_manifest() {
        // Create a valid manifest JSON that the ManifestTestProvider will return
        let manifest_json = r#"{
            "lib_url": "lib://flowstdlib",
            "metadata": {
                "name": "flowstdlib",
                "version": "0.0.0",
                "description": "test",
                "authors": ["me"]
            },
            "locators": {},
            "source_urls": {}
        }"#;

        let payload = Payload {
            job_id: 0,
            input_set: vec![],
            implementation_url: Url::parse("lib://flowstdlib/math/add")
                .expect("Could not parse Url"),
        };

        let loaded_implementations =
            Arc::new(RwLock::new(HashMap::<Url, Arc<dyn Implementation>>::new()));
        let loaded_lib_manifests =
            Arc::new(RwLock::new(HashMap::<Url, (LibraryManifest, Url)>::new()));
        let provider = Arc::new(ManifestTestProvider {
            test_content: manifest_json,
        }) as Arc<dyn Provider>;
        let context = zmq::Context::new();
        let results_sink = context
            .socket(zmq::PUSH)
            .expect("Could not create PUSH end of results-sink socket");
        results_sink
            .connect("tcp://127.0.0.1:3460")
            .expect("Could not connect to PUSH end of results-sink socket");

        let result = super::execute_job(
            &provider,
            &payload,
            &results_sink,
            "test executor",
            &loaded_implementations,
            &loaded_lib_manifests,
        );

        assert!(
            result.is_err(),
            "Should error when implementation is not in the manifest"
        );
        let err_msg = result.expect_err("Expected an error").to_string();
        assert!(
            err_msg.contains("Could not find implementation for"),
            "Error should mention missing locator, got: {err_msg}"
        );
    }

    #[test]
    fn execute_job_context_impl_not_in_manifest() {
        // Create a valid manifest for a context library
        let manifest_json = r#"{
            "lib_url": "context://stdio",
            "metadata": {
                "name": "stdio",
                "version": "0.0.0",
                "description": "test context",
                "authors": ["me"]
            },
            "locators": {},
            "source_urls": {}
        }"#;

        let payload = Payload {
            job_id: 0,
            input_set: vec![],
            implementation_url: Url::parse("context://stdio/stdout").expect("Could not parse Url"),
        };

        let loaded_implementations =
            Arc::new(RwLock::new(HashMap::<Url, Arc<dyn Implementation>>::new()));
        let loaded_lib_manifests =
            Arc::new(RwLock::new(HashMap::<Url, (LibraryManifest, Url)>::new()));
        let provider = Arc::new(ManifestTestProvider {
            test_content: manifest_json,
        }) as Arc<dyn Provider>;
        let context = zmq::Context::new();
        let results_sink = context
            .socket(zmq::PUSH)
            .expect("Could not create PUSH end of results-sink socket");
        results_sink
            .connect("tcp://127.0.0.1:3461")
            .expect("Could not connect to PUSH end of results-sink socket");

        let result = super::execute_job(
            &provider,
            &payload,
            &results_sink,
            "test executor",
            &loaded_implementations,
            &loaded_lib_manifests,
        );

        assert!(
            result.is_err(),
            "Should error when context implementation is not in the manifest"
        );
    }

    #[test]
    fn execute_job_lib_preloaded_but_impl_missing() {
        // Pre-load a manifest with no locators, then try to execute a job
        // referencing an implementation that doesn't exist in it
        let lib_url = Url::parse("lib://flowstdlib").expect("Could not parse lib url");
        let manifest = LibraryManifest::new(lib_url.clone(), test_meta_data());
        let resolved_url =
            Url::parse("file://fake/flowstdlib/location").expect("Could not parse Url");

        let loaded_implementations =
            Arc::new(RwLock::new(HashMap::<Url, Arc<dyn Implementation>>::new()));
        let loaded_lib_manifests =
            Arc::new(RwLock::new(HashMap::<Url, (LibraryManifest, Url)>::new()));

        // Pre-load the manifest
        {
            let mut manifests = loaded_lib_manifests
                .write()
                .expect("Could not write to manifests");
            manifests.insert(lib_url, (manifest, resolved_url));
        }

        let payload = Payload {
            job_id: 0,
            input_set: vec![],
            implementation_url: Url::parse("lib://flowstdlib/math/add")
                .expect("Could not parse Url"),
        };

        let provider = Arc::new(TestProvider { test_content: "" }) as Arc<dyn Provider>;
        let context = zmq::Context::new();
        let results_sink = context
            .socket(zmq::PUSH)
            .expect("Could not create PUSH end of results-sink socket");
        results_sink
            .connect("tcp://127.0.0.1:3462")
            .expect("Could not connect to PUSH end of results-sink socket");

        let result = super::execute_job(
            &provider,
            &payload,
            &results_sink,
            "test executor",
            &loaded_implementations,
            &loaded_lib_manifests,
        );

        assert!(
            result.is_err(),
            "Should error when implementation is not in the pre-loaded manifest"
        );
        let err_msg = result.expect_err("Expected an error").to_string();
        assert!(
            err_msg.contains("Could not find implementation for"),
            "Error should mention missing locator, got: {err_msg}"
        );
    }

    #[test]
    fn get_lib_manifest_tuple_loads_from_provider() {
        // Test that get_lib_manifest_tuple can load a manifest from the provider
        // when it's not already in the loaded manifests map
        let manifest_json = r#"{
            "lib_url": "lib://testlib",
            "metadata": {
                "name": "testlib",
                "version": "1.0.0",
                "description": "a test lib",
                "authors": ["tester"]
            },
            "locators": {},
            "source_urls": {}
        }"#;

        let provider = Arc::new(ManifestTestProvider {
            test_content: manifest_json,
        }) as Arc<dyn Provider>;
        let loaded_lib_manifests =
            Arc::new(RwLock::new(HashMap::<Url, (LibraryManifest, Url)>::new()));
        let lib_root_url = Url::parse("lib://testlib").expect("Could not parse lib url");

        let result = super::get_lib_manifest_tuple(&provider, &loaded_lib_manifests, &lib_root_url);

        assert!(
            result.is_ok(),
            "Should successfully load manifest from provider"
        );
        let (manifest, _resolved_url) = result.expect("Could not get manifest tuple");
        assert_eq!(manifest.metadata.name, "testlib");
        assert_eq!(manifest.metadata.version, "1.0.0");
    }

    #[test]
    fn get_lib_manifest_tuple_uses_cached() {
        // Test that get_lib_manifest_tuple returns a cached manifest
        // rather than loading from provider
        let lib_url = Url::parse("lib://cachedlib").expect("Could not parse lib url");
        let cached_manifest = LibraryManifest::new(
            lib_url.clone(),
            MetaData {
                name: "cachedlib".into(),
                version: "2.0.0".into(),
                description: "cached".into(),
                authors: vec!["cacher".into()],
            },
        );
        let cached_resolved = Url::parse("file://cached/location").expect("Could not parse Url");

        let loaded_lib_manifests =
            Arc::new(RwLock::new(HashMap::<Url, (LibraryManifest, Url)>::new()));
        {
            let mut manifests = loaded_lib_manifests
                .write()
                .expect("Could not write to manifests");
            manifests.insert(lib_url.clone(), (cached_manifest, cached_resolved.clone()));
        }

        // Provider returns different content - but should not be used since manifest is cached
        let provider = Arc::new(TestProvider {
            test_content: "invalid json",
        }) as Arc<dyn Provider>;

        let result = super::get_lib_manifest_tuple(&provider, &loaded_lib_manifests, &lib_url);

        assert!(
            result.is_ok(),
            "Should return cached manifest without calling provider"
        );
        let (manifest, resolved_url) = result.expect("Could not get manifest tuple");
        assert_eq!(
            manifest.metadata.name, "cachedlib",
            "Should return the cached manifest"
        );
        assert_eq!(
            manifest.metadata.version, "2.0.0",
            "Should return the cached manifest version"
        );
        assert_eq!(
            resolved_url, cached_resolved,
            "Should return the cached resolved URL"
        );
    }

    #[test]
    fn get_lib_manifest_tuple_provider_returns_invalid_json() {
        // Test that get_lib_manifest_tuple returns an error when the provider
        // returns invalid manifest content
        let provider = Arc::new(ManifestTestProvider {
            test_content: "not valid json at all",
        }) as Arc<dyn Provider>;
        let loaded_lib_manifests =
            Arc::new(RwLock::new(HashMap::<Url, (LibraryManifest, Url)>::new()));
        let lib_root_url = Url::parse("lib://badlib").expect("Could not parse lib url");

        let result = super::get_lib_manifest_tuple(&provider, &loaded_lib_manifests, &lib_root_url);

        assert!(
            result.is_err(),
            "Should error when provider returns invalid manifest content"
        );
    }

    #[test]
    fn execute_job() {
        let job1 = Job {
            process_id: 1,
            #[cfg(feature = "debugger")]
            function_name: String::new(),
            parent_id: 0,
            connections: vec![],
            payload: Payload {
                job_id: 0,
                input_set: vec![],
                implementation_url: Url::parse("lib://flowstdlib/math/add")
                    .expect("Could not parse Url"),
            },
            result: Ok((None, false)),
            ttl: None,
            attempt: 1,
        };

        let job2 = Job {
            process_id: 1,
            #[cfg(feature = "debugger")]
            function_name: String::new(),
            parent_id: 0,
            connections: vec![],
            payload: Payload {
                job_id: 0,
                input_set: vec![],
                implementation_url: Url::parse("context://stdio/stdout")
                    .expect("Could not parse Url"),
            },
            result: Ok((None, false)),
            ttl: None,
            attempt: 1,
        };

        let job3 = Job {
            process_id: 1,
            #[cfg(feature = "debugger")]
            function_name: String::new(),
            parent_id: 0,
            connections: vec![],
            payload: Payload {
                job_id: 0,
                input_set: vec![],
                implementation_url: Url::parse("file://fake/path").expect("Could not parse Url"),
            },
            result: Ok((None, false)),
            ttl: None,
            attempt: 1,
        };

        for job in [job1, job2, job3] {
            let loaded_implementations =
                Arc::new(RwLock::new(HashMap::<Url, Arc<dyn Implementation>>::new()));
            let loaded_lib_manifests =
                Arc::new(RwLock::new(HashMap::<Url, (LibraryManifest, Url)>::new()));
            let provider = Arc::new(TestProvider { test_content: "" }) as Arc<dyn Provider>;
            let context = zmq::Context::new();
            let results_sink = context
                .socket(zmq::PUSH)
                .expect("Could not createPUSH end of results-sink socket");
            results_sink
                .connect("tcp://127.0.0.1:3458")
                .expect("Could not connect to PULL end of results-sink socket");

            assert!(super::execute_job(
                &provider,
                &job.payload,
                &results_sink,
                "test executor",
                &loaded_implementations,
                &loaded_lib_manifests,
            )
            .is_err());
        }
    }

    /// A trivial native implementation that returns its first input.
    struct EchoImpl;

    impl Implementation for EchoImpl {
        fn run(
            &self,
            inputs: &[serde_json::Value],
        ) -> flowcore::errors::Result<(Option<serde_json::Value>, bool)> {
            Ok((inputs.first().cloned(), false))
        }
    }

    /// Helper: pre-load a native implementation into the maps so tests can
    /// call `get_or_load_implementation` / `execute_job_to_string` without
    /// needing a real provider or manifest.
    #[allow(clippy::type_complexity)]
    fn preloaded_maps(
        url: &Url,
    ) -> (
        Arc<RwLock<HashMap<Url, Arc<dyn Implementation>>>>,
        Arc<RwLock<HashMap<Url, (LibraryManifest, Url)>>>,
    ) {
        let mut impls = HashMap::<Url, Arc<dyn Implementation>>::new();
        impls.insert(url.clone(), Arc::new(EchoImpl));
        (
            Arc::new(RwLock::new(impls)),
            Arc::new(RwLock::new(HashMap::new())),
        )
    }

    #[test]
    fn get_or_load_returns_cached_implementation() {
        let url = Url::parse("context://test/echo").expect("bad url");
        let (loaded_impls, loaded_manifests) = preloaded_maps(&url);
        let provider = Arc::new(TestProvider { test_content: "" }) as Arc<dyn Provider>;
        let payload = Payload {
            job_id: 1,
            input_set: vec![],
            implementation_url: url.clone(),
        };

        // First call — should find the pre-loaded implementation
        let impl1 = super::get_or_load_implementation(
            &provider,
            &payload,
            &loaded_impls,
            &loaded_manifests,
        )
        .expect("first get_or_load failed");

        // Second call — should return the same Arc (no re-loading)
        let impl2 = super::get_or_load_implementation(
            &provider,
            &payload,
            &loaded_impls,
            &loaded_manifests,
        )
        .expect("second get_or_load failed");

        assert!(Arc::ptr_eq(&impl1, &impl2), "should return cached Arc");
    }

    #[test]
    fn execute_job_to_string_returns_serialized_result() {
        let url = Url::parse("context://test/echo").expect("bad url");
        let (loaded_impls, loaded_manifests) = preloaded_maps(&url);
        let provider = Arc::new(TestProvider { test_content: "" }) as Arc<dyn Provider>;
        let payload = Payload {
            job_id: 42,
            input_set: vec![serde_json::json!("hello")],
            implementation_url: url,
        };

        let serialized = super::execute_job_to_string(
            &provider,
            &payload,
            "test",
            &loaded_impls,
            &loaded_manifests,
        )
        .expect("execute_job_to_string failed");

        // The serialized string should be a JSON tuple (job_id, Result)
        let parsed: (usize, Result<(Option<serde_json::Value>, bool)>) =
            serde_json::from_str(&serialized).expect("could not parse result JSON");
        assert_eq!(parsed.0, 42, "job_id should match");
        let (value, run_again) = parsed.1.expect("inner result should be Ok");
        assert_eq!(value, Some(serde_json::json!("hello")));
        assert!(!run_again);
    }

    #[test]
    fn get_or_load_fails_for_unknown_implementation() {
        let url = Url::parse("context://nonexistent/function").expect("bad url");
        let loaded_impls = Arc::new(RwLock::new(HashMap::<Url, Arc<dyn Implementation>>::new()));
        let loaded_manifests = Arc::new(RwLock::new(HashMap::<Url, (LibraryManifest, Url)>::new()));
        let provider = Arc::new(TestProvider { test_content: "" }) as Arc<dyn Provider>;
        let payload = Payload {
            job_id: 1,
            input_set: vec![],
            implementation_url: url,
        };

        let result = super::get_or_load_implementation(
            &provider,
            &payload,
            &loaded_impls,
            &loaded_manifests,
        );
        assert!(result.is_err(), "should fail for unknown implementation");
    }

    /// `get_or_load_implementation` loads from a pre-populated manifest when
    /// the implementation is not yet cached, then caches it for later calls.
    #[test]
    fn get_or_load_loads_from_manifest_and_caches() {
        let impl_url = Url::parse("context://test/echo").expect("bad url");
        let lib_url = Url::parse("context://").expect("bad url");

        // Build a manifest containing our EchoImpl at the impl_url
        let mut manifest = LibraryManifest::new(lib_url.clone(), test_meta_data());
        manifest.locators.insert(
            impl_url.clone(),
            flowcore::model::lib_manifest::ImplementationLocator::Native(Arc::new(EchoImpl)),
        );
        let resolved = Url::parse("memory://").expect("bad url");

        // Pre-populate lib manifests but NOT the implementations map
        let loaded_impls = Arc::new(RwLock::new(HashMap::<Url, Arc<dyn Implementation>>::new()));
        let mut manifests_map = HashMap::new();
        manifests_map.insert(lib_url, (manifest, resolved));
        let loaded_manifests = Arc::new(RwLock::new(manifests_map));

        let provider = Arc::new(TestProvider { test_content: "" }) as Arc<dyn Provider>;
        let payload = Payload {
            job_id: 1,
            input_set: vec![],
            implementation_url: impl_url.clone(),
        };

        // First call: needs_load = true, loads from manifest
        let impl1 = super::get_or_load_implementation(
            &provider,
            &payload,
            &loaded_impls,
            &loaded_manifests,
        )
        .expect("first load from manifest failed");

        // Verify it was cached
        assert!(
            loaded_impls.read().unwrap().contains_key(&impl_url),
            "implementation should be cached after first load"
        );

        // Second call: needs_load = false, returns cached
        let impl2 = super::get_or_load_implementation(
            &provider,
            &payload,
            &loaded_impls,
            &loaded_manifests,
        )
        .expect("second load (cached) failed");

        assert!(Arc::ptr_eq(&impl1, &impl2), "should return same cached Arc");
    }

    /// `execute_job` succeeds with a pre-loaded native implementation and sends
    /// the result to the results sink.
    #[test]
    #[serial]
    fn execute_job_native_succeeds() {
        let url = Url::parse("context://test/echo").expect("bad url");
        let (loaded_impls, loaded_manifests) = preloaded_maps(&url);
        let provider = Arc::new(TestProvider { test_content: "" }) as Arc<dyn Provider>;

        let ports = (
            pick_unused_port().expect("no port"),
            pick_unused_port().expect("no port"),
            pick_unused_port().expect("no port"),
            pick_unused_port().expect("no port"),
        );
        let context = zmq::Context::new();

        // Bind a PULL socket to receive the result
        let results_pull = context.socket(zmq::PULL).expect("PULL socket");
        results_pull
            .bind(&format!("tcp://*:{}", ports.2))
            .expect("bind PULL");

        // Create the PUSH socket that execute_job will use
        let results_sink = context.socket(zmq::PUSH).expect("PUSH socket");
        results_sink
            .connect(&format!("tcp://127.0.0.1:{}", ports.2))
            .expect("connect PUSH");

        let payload = Payload {
            job_id: 7,
            input_set: vec![serde_json::json!(42)],
            implementation_url: url,
        };

        let keep = super::execute_job(
            &provider,
            &payload,
            &results_sink,
            "test",
            &loaded_impls,
            &loaded_manifests,
        )
        .expect("execute_job should succeed");

        assert!(keep, "execute_job should return true to keep processing");

        // Verify the result arrived on the PULL socket
        let msg = results_pull.recv_msg(0).expect("should receive result");
        let msg_str = msg.as_str().expect("result should be str");
        let parsed: (usize, Result<(Option<serde_json::Value>, bool)>) =
            serde_json::from_str(msg_str).expect("should parse result");
        assert_eq!(parsed.0, 7);
        let (value, run_again) = parsed.1.expect("inner should be Ok");
        assert_eq!(value, Some(serde_json::json!(42)));
        assert!(!run_again);
    }

    /// `execute_job_to_string` fails gracefully when the implementation is missing.
    #[test]
    fn execute_job_to_string_fails_for_missing_impl() {
        let url = Url::parse("context://missing/impl").expect("bad url");
        let loaded_impls = Arc::new(RwLock::new(HashMap::<Url, Arc<dyn Implementation>>::new()));
        let loaded_manifests = Arc::new(RwLock::new(HashMap::<Url, (LibraryManifest, Url)>::new()));
        let provider = Arc::new(TestProvider { test_content: "" }) as Arc<dyn Provider>;
        let payload = Payload {
            job_id: 1,
            input_set: vec![],
            implementation_url: url,
        };

        let result = super::execute_job_to_string(
            &provider,
            &payload,
            "test",
            &loaded_impls,
            &loaded_manifests,
        );
        assert!(
            result.is_err(),
            "should fail when implementation is not loaded"
        );
    }

    /// Verify `start_spawning` creates executor threads (they connect to ZMQ and
    /// exit when DONE is sent on the control socket).
    #[test]
    #[serial]
    fn start_spawning_creates_threads() {
        let mut executor = Executor::new();
        let provider = Arc::new(TestProvider { test_content: "" }) as Arc<dyn Provider>;

        let ports = (
            pick_unused_port().expect("no port"),
            pick_unused_port().expect("no port"),
            pick_unused_port().expect("no port"),
            pick_unused_port().expect("no port"),
        );

        let context = zmq::Context::new();
        // Bind sockets that the executor threads will connect to
        let job_socket = context.socket(zmq::PUSH).expect("PUSH");
        job_socket
            .bind(&format!("tcp://*:{}", ports.0))
            .expect("bind job");
        let results_socket = context.socket(zmq::PULL).expect("PULL");
        results_socket
            .bind(&format!("tcp://*:{}", ports.2))
            .expect("bind results");
        let control_socket = context.socket(zmq::PUB).expect("PUB");
        control_socket
            .bind(&format!("tcp://*:{}", ports.3))
            .expect("bind control");

        executor.start_spawning(
            &provider,
            1,
            &format!("tcp://127.0.0.1:{}", ports.0),
            &format!("tcp://127.0.0.1:{}", ports.2),
            &format!("tcp://127.0.0.1:{}", ports.3),
        );

        assert_eq!(executor.executors.len(), 1, "should have 1 executor thread");

        // Give the thread time to connect, then send DONE
        std::thread::sleep(std::time::Duration::from_millis(200));
        control_socket
            .send("DONE".as_bytes(), 0)
            .expect("send DONE");

        // Wait for the thread to exit
        executor.wait();
    }
}

use log::{info, debug, error, trace};
use std::thread;
use std::collections::HashMap;
use std::panic;
use std::sync::{Arc, RwLock};
use url::Url;
use crate::wasm;
use crate::job::Job;
use flowcore::Implementation;
use flowcore::model::lib_manifest::{
    ImplementationLocator::Native, ImplementationLocator::Wasm, LibraryManifest,
};
use flowcore::errors::*;
use flowcore::provider::Provider;

/// An `Executor` struct is used to receive jobs, execute them and return results.
/// It can load libraries and keep track of the `Function` `Implementations` loaded for use
/// in job execution.
pub struct Executor {
    // HashMap of library manifests already loaded. The key is the library reference Url
    // (e.g. lib:://flowstdlib) and the entry is a tuple of the LibraryManifest
    // and the resolved Url of where the manifest was read from
    loaded_lib_manifests: Arc<RwLock<HashMap<Url, (LibraryManifest, Url)>>>,
}

impl Executor {
    /// Create a new executor that receives jobs, executes them and returns results.
    pub fn new() -> Result<Self> {
        Ok(Executor{
            loaded_lib_manifests: Arc::new(RwLock::new(HashMap::<Url, (LibraryManifest, Url)>::new()))
        })
    }

    /// Add a library manifest so that it can be used later on to load implementations that are
    /// required to execute jobs. Also provide the Url that the library url resolves to, so that
    /// later it can be used when resolving the locations of implementations in this library.
    pub fn add_lib(
        &mut self,
        lib_manifest: LibraryManifest,
        resolved_url: Url
    ) -> Result<()> {
        let mut lib_manifests = self.loaded_lib_manifests.write()
            .map_err(|_| "Could not gain write access to loaded library manifests map")?;

        debug!("Manifest of library '{}' loaded from '{}' and added to Executor",
            lib_manifest.lib_url, resolved_url);

        lib_manifests.insert(lib_manifest.lib_url.clone(), (lib_manifest, resolved_url));

        Ok(())
    }

    /// Start executing jobs, specifying:
    /// - the `Provider` to use to fetch implementation content
    /// - the number of executor threads
    /// - the address of the job socket to get jobs from
    /// - the address of the results socket to return results from executed jobs to
    pub fn start(&mut self,
                 provider: Arc<dyn Provider>,
                 number_of_executors: usize,
                 job_service: &str,
                 results_service: &str,
    ) {
        let loaded_implementations = Arc::new(RwLock::new(HashMap::<Url, Arc<dyn Implementation>>::new()));

        info!("Starting {number_of_executors} executor threads");
        for executor_number in 0..number_of_executors {
            let thread_provider = provider.clone();
            let thread_context = zmq::Context::new();
            let thread_implementations = loaded_implementations.clone();
            let thread_loaded_manifests = self.loaded_lib_manifests.clone();
            let results_sink = results_service.into();
            let job_source = job_service.into();
            thread::spawn(move || {
                trace!("Executor #{executor_number} entering execution loop");
                if let Err(e) = execution_loop(
                    thread_provider,
                    format!("Executor #{executor_number}"),
                    thread_context,
                    thread_implementations,
                    thread_loaded_manifests,
                    job_source,
                    results_sink,
                ) {
                    error!("Execution loop error: {e}");
                }
            });
        }
    }
}

fn execution_loop(
    provider: Arc<dyn Provider>,
    name: String,
    context: zmq::Context,
    loaded_implementations: Arc<RwLock<HashMap<Url, Arc<dyn Implementation>>>>,
    loaded_lib_manifests: Arc<RwLock<HashMap<Url, (LibraryManifest, Url)>>>,
    job_service: String,
    results_service: String,
) -> Result<()> {
    let job_source = context.socket(zmq::PULL)
        .map_err(|e|
            format!("Could not create PULL end of job socket: {e}"))?;
    job_source.connect(&job_service)
        .map_err(|e|
            format!("Could not connect to PULL end of job socket: '{job_service}' {e}", ))?;

    let results_sink = context.socket(zmq::PUSH)
        .map_err(|e| format!("Could not create PUSH end of results socket: {e}"))?;
    results_sink.connect(&results_service)
        .map_err(|e| format!("Could not connect to PUSH end of results socket: {e}"))?;

    let mut process_jobs = true;

    set_panic_hook();

    while process_jobs {
        trace!("{name} waiting for a job to execute");
        match job_source.recv_msg(0) {
            Ok(msg) => {
                let message_string = msg.as_str().ok_or("Could not get message as str")?;
                let mut job: Job = serde_json::from_str(message_string)
                    .map_err(|_| "Could not deserialize Message to Job")?;

                trace!("Job #{}: Received for execution: {}", job.job_id, job);
                match execute_job(provider.clone(),
                                  &mut job,
                                  &results_sink,
                                  &name,
                                  loaded_implementations.clone(),
                                  loaded_lib_manifests.clone()) {
                    Ok(keep_processing) => process_jobs = keep_processing,
                    Err(e) => error!("{}", e)
                }
            }
            Err(e) => {
                error!("Error while polling for Jobs to execute: {e}");
            }
        }
    }

    Ok(())
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

/// Return Ok(keep_processing) flag as true or false to keep processing
fn execute_job(
    provider: Arc<dyn Provider>,
    job: &mut Job,
    results_sink: &zmq::Socket,
    name: &str,
    loaded_implementations: Arc<RwLock<HashMap<Url, Arc<dyn Implementation>>>>,
    loaded_lib_manifests: Arc<RwLock<HashMap<Url, (LibraryManifest, Url)>>>,
) -> Result<bool> {
    // TODO see if we can avoid write access until we know it's needed
    let mut implementations = loaded_implementations.write()
        .map_err(|_| "Could not gain read access to loaded implementations map")?;
    if implementations.get(&job.implementation_url).is_none() {
        trace!("Implementation '{}' is not loaded", job.implementation_url);
        let implementation = match job.implementation_url.scheme() {
            "lib" => {
                let mut lib_root_url = job.implementation_url.clone();
                lib_root_url.set_path("");
                load_referenced_implementation(provider,
                                               lib_root_url,
                                               loaded_lib_manifests,
                                               &job.implementation_url)?
            },
            "context" => {
                let mut lib_root_url = job.implementation_url.clone();
                let _ = lib_root_url.set_host(Some(""));
                lib_root_url.set_path("");
                load_referenced_implementation(provider,
                                               lib_root_url,
                                               loaded_lib_manifests,
                                               &job.implementation_url)?
            },
            "file" => Arc::new(wasm::load(&*provider, &job.implementation_url)?),
            _ => bail!("Unsupported scheme on implementation_url")
        };
        implementations.insert(job.implementation_url.clone(), implementation);
        trace!("Implementation '{}' added to executor", job.implementation_url);
    }

    let implementation = implementations.get(&job.implementation_url)
        .ok_or("Could not find implementation")?;

    trace!("Job #{}: Started executing on '{name}'", job.job_id);
    job.result = implementation.run(&job.input_set);
    trace!("Job #{}: Finished executing on '{name}'", job.job_id);

    results_sink.send(serde_json::to_string(&job)?.as_bytes(), 0)
        .map_err(|_| "Could not send result of Job")?;

    Ok(true)
}

// Load a context or library implementation
fn load_referenced_implementation(
    provider: Arc<dyn Provider>,
    lib_root_url: Url,
    loaded_lib_manifests: Arc<RwLock<HashMap<Url, (LibraryManifest, Url)>>>,
    implementation_url: &Url
) -> Result<Arc<dyn Implementation>> {
    let (lib_manifest, resolved_lib_url) = get_lib_manifest_tuple(provider.clone(), loaded_lib_manifests, &lib_root_url)?;

    let locator = lib_manifest
        .locators
        .get(implementation_url)
        .ok_or(format!(
            "Could not find ImplementationLocator for '{}' in library",
            implementation_url
        ))?;

    // find the implementation we need from the locator
    let implementation = match locator {
        Wasm(wasm_source_relative) => {
            // Path to the wasm source could be relative to the URL where we loaded the manifest from
            let wasm_url = resolved_lib_url
                .join(wasm_source_relative)
                .map_err(|e| e.to_string())?;
            debug!("Attempting to load wasm from source file: '{}'", wasm_url);
            // Wasm implementation being added. Wrap it with the Wasm Native Implementation
            let wasm_executor = wasm::load(&*provider as &dyn Provider, &wasm_url)?;
            Arc::new(wasm_executor) as Arc<dyn Implementation>
        }
        Native(native_impl) => native_impl.clone(),
    };

    Ok(implementation)
}

// Get the tuple of the lib manifest and the url from where it was loaded from
fn get_lib_manifest_tuple(
    provider: Arc<dyn Provider>,
    loaded_lib_manifests: Arc<RwLock<HashMap<Url, (LibraryManifest, Url)>>>,
    lib_root_url: &Url,
) -> Result<(LibraryManifest, Url)> {
    let mut lib_manifests = loaded_lib_manifests.write()
        .map_err(|_| "Could not get write access to the loaded lib manifests")?;

    if lib_manifests.get(lib_root_url).is_none() {
        info!("Attempting to load library manifest'{}'", lib_root_url);
        let manifest_tuple =
            LibraryManifest::load(&*provider as &dyn Provider, lib_root_url)
                .chain_err(|| format!("Could not load library with root url: '{lib_root_url}'"))?;
        lib_manifests
            .insert(lib_root_url.clone(), manifest_tuple);
    }

    // TODO avoid this clone and return references
    lib_manifests
        .get(lib_root_url)
        .ok_or_else(|| "Could not find (supposedly already loaded) library manifest".into())
        .clone().cloned()
}

#[cfg(test)]
mod test {
    use url::Url;
    use super::Executor;
    use flowcore::model::metadata::MetaData;
    use flowcore::model::lib_manifest::LibraryManifest;
    use flowcore::provider::Provider;
    use flowcore::errors::Result;
    use crate::job::Job;
    use std::sync::{Arc, RwLock};
    use std::collections::HashMap;
    use flowcore::Implementation;

    fn test_meta_data() -> MetaData {
        MetaData {
            name: "test".into(),
            version: "0.0.0".into(),
            description: "a test".into(),
            authors: vec!["me".into()],
        }
    }

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

    #[test]
    fn test_constructor() {
        let executor = Executor::new();
        assert!(executor.is_ok())
    }

    #[test]
    fn add_a_lib() {
        let library = LibraryManifest::new(
            Url::parse("lib://testlib").expect("Could not parse lib url"),
            test_meta_data(),
        );

        let mut executor = Executor::new().expect("New failed");
        assert!(executor.add_lib(library,
                         Url::parse("file://fake/lib/location")
                             .expect("Could not parse Url")).is_ok());
    }

    #[test]
    fn execute_job() {
        let job1 = Job {
            job_id: 0,
            function_id: 1,
            flow_id: 0,
            input_set: vec![],
            connections: vec![],
            implementation_url: Url::parse("lib://flowstdlib/math/add").expect("Could not parse Url"),
            result: Ok((None, false)),
        };

        let job2 = Job {
            job_id: 0,
            function_id: 1,
            flow_id: 0,
            input_set: vec![],
            connections: vec![],
            implementation_url: Url::parse("context://stdio/stdout").expect("Could not parse Url"),
            result: Ok((None, false)),
        };

        let job3 = Job {
            job_id: 0,
            function_id: 1,
            flow_id: 0,
            input_set: vec![],
            connections: vec![],
            implementation_url: Url::parse("file://fake/path").expect("Could not parse Url"),
            result: Ok((None, false)),
        };

        for mut job in vec![job1, job2, job3] {
            let loaded_implementations = Arc::new(RwLock::new(HashMap::<Url, Arc<dyn Implementation>>::new()));
            let loaded_lib_manifests = Arc::new(RwLock::new(HashMap::<Url, (LibraryManifest, Url)>::new()));
            let provider = Arc::new(TestProvider{test_content: ""});
            let context = zmq::Context::new();
            let results_sink = context.socket(zmq::PUSH)
                .expect("Could not createPUSH end of results-sink socket");
            results_sink.connect("tcp://127.0.0.1:3458")
                .expect("Could not connect to PULL end of results-sink socket");

            assert!(super::execute_job(provider,
                                       &mut job,
                                       &results_sink,
                                       "test executor",
                                       loaded_implementations,
                                       loaded_lib_manifests,
            ).is_err());
        }
    }
}
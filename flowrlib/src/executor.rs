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

/// It can load libraries and keep track of the `Function` `Implementations` used in execution.
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

    /// Add a library's manifest to the set of those to reference later. This is mainly for use
    /// prior to running a flow to ensure that the preferred libraries (e.g. flowstdlib native
    /// version) is pre-loaded.
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
    ///- the `Provider` to use to fetch implementation content
    ///- optional timeout for waiting for results
    ///- the number of executor threads
    /// - whether to poll for context jobs also
    pub fn start(&mut self,
                 provider: Arc<dyn Provider>,
                 number_of_executors: usize,
                 job_source_name: Option<&str>,
                 context_job_source_name: Option<&str>,
                 results_sink_name: &str,
    ) -> Result<()> {
        let loaded_implementations = Arc::new(RwLock::new(HashMap::<Url, Arc<dyn Implementation>>::new()));

        info!("Starting {} executor threads", number_of_executors);
        for executor_number in 0..number_of_executors {
            let thread_provider = provider.clone();
            let thread_context = zmq::Context::new();
            let thread_implementations = loaded_implementations.clone();
            let thread_loaded_manifests = self.loaded_lib_manifests.clone();
            let job_source = job_source_name.map(|s| s.into());
            let context_job_source = context_job_source_name.map(|s| s.into());
            let results_sink = results_sink_name.into();
            thread::spawn(move || {
                execution_loop(
                    thread_provider,
                    format!("Executor #{executor_number}"),
                    thread_context,
                    thread_implementations,
                    thread_loaded_manifests,
                    job_source,
                    context_job_source,
                    results_sink,
                ) // clone of Arcs and Sender OK
            });
        }

        Ok(())
    }
}

#[allow(clippy::too_many_arguments)]
fn execution_loop(
    provider: Arc<dyn Provider>,
    name: String,
    context: zmq::Context,
    loaded_implementations: Arc<RwLock<HashMap<Url, Arc<dyn Implementation>>>>,
    loaded_lib_manifests: Arc<RwLock<HashMap<Url, (LibraryManifest, Url)>>>,
    job_source_name: Option<String>,
    context_job_source_name: Option<String>,
    results_sink_name: String,
) -> Result<()> {
    let mut sockets : Vec<&zmq::Socket> = vec![];
    let mut items : Vec<zmq::PollItem> = vec![];

    let job_source : zmq::Socket;
    if let Some(job_source_n) = job_source_name {
        job_source = context.socket(zmq::PULL)
            .map_err(|_| "Could not create PULL end of job-source socket")?;
        job_source.connect(&job_source_n)
            .map_err(|_| "Could not bind to PULL end of job-source socket")?;
        sockets.push(&job_source);
        items.push(job_source.as_poll_item(zmq::POLLIN));
    }

    let context_job_source : zmq::Socket;
    if let Some(context_job_source_n) = context_job_source_name {
        context_job_source = context.socket(zmq::PULL)
            .map_err(|_| "Could not create PULL end of context-job-source socket")?;
        context_job_source.connect(&context_job_source_n)
            .map_err(|_| "Could not bind to PULL end of context-job-source  socket")?;
        sockets.push(&context_job_source);
        items.push(context_job_source.as_poll_item(zmq::POLLIN));
    }

    let results_sink = context.socket(zmq::PUSH)
        .map_err(|_| "Could not createPUSH end of results-sink socket")?;
    results_sink.connect(&results_sink_name)
        .map_err(|_| "Could not connect to PULL end of results-sink socket")?;

    let mut process_jobs = true;

    set_panic_hook();

    while process_jobs {
        trace!("{name} waiting for a job to execute");
        zmq::poll(&mut items, -1).map_err(|_| "Error while polling for Jobs to execute")?;

        for (index, item) in items.iter().enumerate() {
            if item.is_readable() {
                let socket = sockets.get(index).ok_or("Could not get that socket")?;
                let msg = socket.recv_msg(0).map_err(|_| "Error receiving Job for execution")?;
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
                trace!("{name} finished executing job");
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
        trace!("Implementation at '{}' is not loaded", job.implementation_url);
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

    const JOB_SOURCE_NAME: &str  = "tcp://127.0.0.1:3456";
    const CONTEXT_JOB_SOURCE_NAME: &str  = "tcp://127.0.0.1:3457";
    const RESULTS_SINK_NAME: &str  = "tcp://127.0.0.1:3458";

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
    fn start_zero_executors() {
        let mut executor = Executor::new().expect("Could not create executor");
        let provider = Arc::new(TestProvider{test_content: ""});
        assert!(executor.start(provider, 0,
                               Some(JOB_SOURCE_NAME),
                               Some(CONTEXT_JOB_SOURCE_NAME),
                               RESULTS_SINK_NAME).is_ok());
    }

    #[test]
    fn start_one_executor() {
        let mut executor = Executor::new().expect("Could not create executor");
        let provider = Arc::new(TestProvider{test_content: ""});
        assert!(executor.start(provider, 1,
                               Some(JOB_SOURCE_NAME),
                               Some(CONTEXT_JOB_SOURCE_NAME),
                               RESULTS_SINK_NAME).is_ok());
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
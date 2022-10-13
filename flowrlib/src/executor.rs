use std::collections::HashMap;
use std::panic;
use std::sync::{Arc, mpsc, Mutex, RwLock};
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::time::Duration;

use log::{debug, error, info, trace};
use url::Url;

use flowcore::errors::*;
use flowcore::Implementation;
use flowcore::model::lib_manifest::{
    ImplementationLocator::Native, ImplementationLocator::Wasm, LibraryManifest,
};
use flowcore::provider::Provider;

use crate::job::Job;
use crate::wasm;

/// Executor structure holds information required to send jobs for execution and receive results back
/// It can load a compiled `Flow` from it's `FlowManifest`, loading the required
/// libraries needed by the flow and keeping track of the `Function` `Implementations` that
/// will be used to execute it.
pub struct Executor {
    // A channel used to send Jobs out for execution locally
    job_sender: Sender<Job>,
    // A channel used to receive Jobs back after local execution (now including the job's output)
    results_receiver: Receiver<Job>,
    // The timeout for waiting for results back from jobs being executed
    job_timeout: Option<Duration>,
    // HashMap of library manifests already loaded. The key is the library reference Url
    // (e.g. lib:://flowstdlib) and the entry is a tuple of the LibraryManifest
    // and the resolved Url of where the manifest was read from
    loaded_lib_manifests: Arc<RwLock<HashMap<Url, (LibraryManifest, Url)>>>,
}

/// Struct that takes care of execution of jobs, sending jobs for execution and receiving results
impl Executor {
    /// Create a new `Executor` specifying the number of local executor threads and a timeout
    /// for reception of results
    pub fn new(provider: Arc<dyn Provider>, number_of_executors: usize, job_timeout: Option<Duration>) -> Self {
        let (job_sender, job_receiver) = mpsc::channel();
        let (results_sender, results_receiver) = mpsc::channel();

        info!("Starting {} local executor threads", number_of_executors);
        let shared_job_receiver = Arc::new(Mutex::new(job_receiver));

//        start_p2p_sender_receiver(shared_job_receiver, results_sender);

        let loaded_implementations = Arc::new(RwLock::new(HashMap::<Url, Arc<dyn Implementation>>::new()));
        let loaded_lib_manifests = Arc::new(RwLock::new(HashMap::<Url, (LibraryManifest, Url)>::new()));

        start_local_executors(provider, number_of_executors, shared_job_receiver, results_sender,
                              loaded_implementations, loaded_lib_manifests.clone());

        Executor {
            job_sender,
            results_receiver,
            job_timeout,
            loaded_lib_manifests,
        }
    }

    /// Set the timeout to use when waiting for job results after execution
    pub fn set_timeout(&mut self, timeout: Option<Duration>) {
        self.job_timeout = timeout;
    }

    /// Wait for, then return the next Job with results returned from executors
    pub fn get_next_result(&mut self) -> Result<Job> {
        match self.job_timeout {
            Some(t) => self.results_receiver.recv_timeout(t)
                .chain_err(|| "Timeout while waiting for Job result"),
            None => self.results_receiver.recv()
                .chain_err(|| "Error while trying to receive Job results")
        }
    }

    // Send a `Job` for execution to executors
    pub(crate) fn send_job_for_execution(&mut self, job: &Job) -> Result<()> {
        self.job_sender
            .send(job.clone())
            .chain_err(|| "Sending of job for execution failed")?;

        trace!(
            "Job #{}: Sent for execution of Function #{}",
            job.job_id,
            job.function_id
        );

        Ok(())
    }

    /// Add a library's manifest to the set of those to reference later. This is mainly for use
    /// prior to running a flow to ensure that the preferred libraries (e.g. flowstdlib native
    /// version) is pre-loaded.
    pub fn add_lib(
        &mut self,
        lib_manifest: LibraryManifest,
        resolved_url: Url
    ) -> Result<()> {
        let mut lib_manifests = self.loaded_lib_manifests.try_write()
            .map_err(|_| "Could not gain write access to loaded library manifests map")?;

        debug!("Manifest of library {} loaded from {} and added to Executor",
            lib_manifest.lib_url, resolved_url);

        lib_manifests.insert(lib_manifest.lib_url.clone(), (lib_manifest, resolved_url));

        Ok(())
    }
}

// Start a number of executor threads that all listen on the 'job_rx' channel for
// Jobs to execute and return the Outputs on the 'output_tx' channel
fn start_local_executors(
    provider: Arc<dyn Provider>,
    number_of_executors: usize,
    shared_job_receiver: Arc<Mutex<Receiver<Job>>>,
    job_sender: Sender<Job>,
    loaded_implementations: Arc<RwLock<HashMap<Url, Arc<dyn Implementation>>>>,
    loaded_lib_manifests: Arc<RwLock<HashMap<Url, (LibraryManifest, Url)>>>,
) {
    for executor_number in 0..number_of_executors {
        create_executor_thread(
                    provider.clone(),
            format!("Executor #{}", executor_number),
            shared_job_receiver.clone(),
            job_sender.clone(),
            loaded_implementations.clone(),
            loaded_lib_manifests.clone(),
        ); // clone of Arcs and Sender OK
    }
}

fn create_executor_thread(
                    provider: Arc<dyn Provider>,
                    name: String,
                   job_receiver: Arc<Mutex<Receiver<Job>>>,
                   job_sender: Sender<Job>,
                   loaded_implementations: Arc<RwLock<HashMap<Url, Arc<dyn Implementation>>>>,
                   loaded_lib_manifests: Arc<RwLock<HashMap<Url, (LibraryManifest, Url)>>>,
) {
    let builder = thread::Builder::new();
    let _ = builder.spawn(move || {
        let mut process_jobs = true;

        set_panic_hook();

        while process_jobs {
            match get_and_execute_job(provider.clone(), &job_receiver, &job_sender,
                                        &name,
                                        loaded_implementations.clone(),
                                            loaded_lib_manifests.clone()) {
                Ok(keep_processing) => process_jobs = keep_processing,
                Err(e) => error!("{}", e)
            }
        }
    });
}

// Replace the standard panic hook with one that just outputs the file and line of any panic.
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

/// Return Ok(keep_processing) flag as true or false to keep processing
fn get_and_execute_job(
    provider: Arc<dyn Provider>,
    job_receiver: &Arc<Mutex<Receiver<Job>>>,
    job_sender: &Sender<Job>,
    name: &str,
    loaded_implementations: Arc<RwLock<HashMap<Url, Arc<dyn Implementation>>>>,
    loaded_lib_manifests: Arc<RwLock<HashMap<Url, (LibraryManifest, Url)>>>,
) -> Result<bool> {
    let guard = job_receiver
        .lock()
        .map_err(|e| format!("Error locking receiver to get job: '{}'", e))?;
    // return Ok(false) to end processing loop if channel is shut down by parent process
    let job = match guard.recv() {
        Ok(j) => j,
        Err(_) => return Ok(false)
    };

    trace!("Job #{} received for execution: {}", job.job_id, job);
    let mut implementations = loaded_implementations.try_write()
        .map_err(|_| "Could not gain write access to loaded implementations map")?;
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
            "file" => resolve_implementation(provider, &job.implementation_url)?,
            _ => bail!("Unsupported scheme on implementation_url")
        };
        implementations.insert(job.implementation_url.clone(), implementation);
        trace!("Implementation '{}' added to executor", job.implementation_url);
    }

    let implementation = implementations.get(&job.implementation_url)
        .ok_or("Could not find implementation")?;

    execute_job(job, job_sender, name, implementation)?;

    Ok(true)
}

fn execute_job(
    mut job: Job,
    job_tx: &Sender<Job>,
    name: &str,
    implementation: &Arc<dyn Implementation>,
) -> Result<()> {
    trace!("Job #{}: Started executing on '{name}'", job.job_id);
    job.result = implementation.run(&job.input_set);
    trace!("Job #{}: Finished executing on '{name}'", job.job_id);
    job_tx.send(job).chain_err(|| "Error sending job result back after execution")
}

// Load a WASM `Implementation` from the `implementation_url` using the supplied `Provider`
fn resolve_implementation(provider: Arc<dyn Provider>,
                          implementation_url: &Url,
) -> Result<Arc<dyn Implementation>> {
    let wasm_executor = wasm::load(&* provider, implementation_url)?;
    Ok(Arc::new(wasm_executor) as Arc<dyn Implementation>)
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

    let mut lib_manifests = loaded_lib_manifests.try_write()
        .map_err(|_| "Could not get write access to the loaded lib manifests")?;

    if lib_manifests.get(lib_root_url).is_none() {
        info!("Attempting to load library manifest'{}'", lib_root_url);
        let manifest_tuple =
            LibraryManifest::load(&*provider as &dyn Provider, lib_root_url).chain_err(|| {
                format!("Could not load library with root url: '{}'", lib_root_url)
            })?;
        lib_manifests
            .insert(lib_root_url.clone(), manifest_tuple);
    }

    let tuple = lib_manifests
        .get(lib_root_url)
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "Could not find (supposedly already loaded) library manifest",
            )
        })?;

    // TODO try and avoid clone
    Ok(tuple.clone())
}
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

use libp2p::gossipsub::MessageId;
use libp2p::gossipsub::{
    Gossipsub, GossipsubEvent, GossipsubMessage, IdentTopic as Topic, MessageAuthenticity,
    ValidationMode,
};
use libp2p::{
    gossipsub, identity,
    mdns::{Mdns, MdnsConfig, MdnsEvent},
    NetworkBehaviour, PeerId, Swarm,
};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use libp2p::futures::executor::block_on;
use libp2p::futures::StreamExt;
use libp2p::swarm::SwarmEvent;

// Create a custom network behaviour that combines Gossipsub and Mdns.
#[derive(NetworkBehaviour)]
#[behaviour(out_event = "MyBehaviourEvent")]
struct MyBehaviour {
    gossipsub: Gossipsub,
    mdns: Mdns,
}

enum MyBehaviourEvent {
    Gossipsub(GossipsubEvent),
    Mdns(MdnsEvent),
}

impl From<GossipsubEvent> for MyBehaviourEvent {
    fn from(event: GossipsubEvent) -> Self {
        MyBehaviourEvent::Gossipsub(event)
    }
}

impl From<MdnsEvent> for MyBehaviourEvent {
    fn from(event: MdnsEvent) -> Self {
        MyBehaviourEvent::Mdns(event)
    }
}

#[allow(unused)]
fn swarm() -> Result<Swarm<MyBehaviour>> {
    // Create a random PeerId
    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());
    println!("Local peer id: {}", local_peer_id);

    // Set up a an encrypted DNS-enabled TCP Transport over the Mplex protocol.
    let transport = block_on(libp2p::development_transport(local_key.clone()))?;

    // To content-address message, we can take the hash of message and use it as an ID.
    // No two messages of the same content will be propagated. TODO review this
    let message_id_fn = |message: &GossipsubMessage| {
        let mut s = DefaultHasher::new();
        message.data.hash(&mut s);
        MessageId::from(s.finish().to_string())
    };

    // Set a custom gossipsub configuration
    let gossipsub_config = gossipsub::GossipsubConfigBuilder::default()
        .heartbeat_interval(Duration::from_secs(10))
        .validation_mode(ValidationMode::Strict)
        .message_id_fn(message_id_fn)
        .build()?;

    // build a gossipsub network behaviour
    let mut gossipsub = Gossipsub::new(MessageAuthenticity::Signed(local_key), gossipsub_config)?;

    // Create a Gossipsub topic
    let topic = Topic::new("flow:jobs");

    // subscribes to our topic
    gossipsub.subscribe(&topic).map_err(|_| "Could not subscribe to the topic")?;

    // Create a Swarm to manage peers and events
    let mdns = block_on(Mdns::new(MdnsConfig::default()))?;
    Ok(Swarm::new(transport, MyBehaviour { gossipsub, mdns }, local_peer_id))
}

#[allow(unused)]
fn swam_loop(swarm: &mut Swarm<MyBehaviour>) -> Result<()> {
    // Listen on all interfaces and whatever port the OS assigns
    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse().map_err(|_| "Could not parse MultiAddress")?)
        .map_err(|_| "Could not listen on 0.0.0.0 MultiAddress")?;

    loop {
        while let Some(event) = block_on(swarm.next()) {
            match event {
                // TODO look at https://docs.rs/libp2p/0.35.1/libp2p/swarm/enum.SwarmEvent.html for other events to listen on
                SwarmEvent::Behaviour(MyBehaviourEvent::Mdns(MdnsEvent::Discovered(list))) => {
                    for (peer_id, _multiaddr) in list {
                        info!("mDNS discovered a new peer: {}", peer_id);
                        swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                    }
                },
                SwarmEvent::Behaviour(MyBehaviourEvent::Gossipsub(GossipsubEvent::Message {
                                                                      propagation_source: _peer_id,
                                                                      message_id: _id,
                                                                      message,
                                                                  })) => {
                    let job = Job::try_from(&message.data)?;
                    println!("Got Job: '{}' from peer: {:?}", job, message.source);
                },
                _ => {}
            }
        }
    }
}

/// Executor structure holds information required to send jobs for execution and receive results back
/// It can load a compiled `Flow` from it's `FlowManifest`, loading the required
/// libraries needed by the flow and keeping track of the `Function` `Implementations` that
/// will be used to execute it.
pub struct Executor {
    // A channel used to send Jobs out for execution locally
    job_sender: Sender<Job>,
    // A shared job receiver that executor threads will pull jobs from
    shared_job_receiver: Arc<Mutex<Receiver<Job>>>,
    // A channel used to receive Jobs back after local execution (now including the job's output)
    results_receiver: Receiver<Job>,
    // A sender for results back from executor threads
    results_sender: Sender<Job>,
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
    pub fn new(job_timeout: Option<Duration>) -> Self {
        let (job_sender, job_receiver) = mpsc::channel();
        let (results_sender, results_receiver) = mpsc::channel();

        let shared_job_receiver = Arc::new(Mutex::new(job_receiver));

        let loaded_lib_manifests = Arc::new(RwLock::new(HashMap::<Url, (LibraryManifest, Url)>::new()));

        Executor {
            job_sender,
            shared_job_receiver,
            results_receiver,
            results_sender,
            job_timeout,
            loaded_lib_manifests,
        }
    }

    /// Start the executors for jobs
    pub fn start(&mut self, provider: Arc<dyn Provider>, number_of_executors: usize) {
        info!("Starting {} local executor threads", number_of_executors);
        self.start_local_executors(provider, number_of_executors);
    }

    // Start a number of executor threads that all listen on the 'job_rx' channel for
    // Jobs to execute and return the Outputs on the 'output_tx' channel
    fn start_local_executors(
        &mut self,
        provider: Arc<dyn Provider>,
        number_of_executors: usize,
    ) {
        let loaded_implementations = Arc::new(RwLock::new(HashMap::<Url, Arc<dyn Implementation>>::new()));

        for executor_number in 0..number_of_executors {
            create_executor_thread(
                provider.clone(),
                format!("Executor #{}", executor_number),
                self.shared_job_receiver.clone(),
                self.results_sender.clone(),
                loaded_implementations.clone(),
                self.loaded_lib_manifests.clone(),
            ); // clone of Arcs and Sender OK
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

fn create_executor_thread(
    provider: Arc<dyn Provider>,
    name: String,
    job_receiver: Arc<Mutex<Receiver<Job>>>,
    results_sender: Sender<Job>,
    loaded_implementations: Arc<RwLock<HashMap<Url, Arc<dyn Implementation>>>>,
    loaded_lib_manifests: Arc<RwLock<HashMap<Url, (LibraryManifest, Url)>>>,
) {
    let builder = thread::Builder::new();
    let _ = builder.spawn(move || {
        set_panic_hook();

        loop {
            let _ = get_and_execute_job(provider.clone(), &job_receiver, &results_sender,
                                        &name,
                                        loaded_implementations.clone(),
                                        loaded_lib_manifests.clone()
                );
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

fn get_and_execute_job(
    provider: Arc<dyn Provider>,
    job_receiver: &Arc<Mutex<Receiver<Job>>>,
    results_sender: &Sender<Job>,
    name: &str,
    loaded_implementations: Arc<RwLock<HashMap<Url, Arc<dyn Implementation>>>>,
    loaded_lib_manifests: Arc<RwLock<HashMap<Url, (LibraryManifest, Url)>>>,
) -> Result<()> {
    let guard = job_receiver
        .lock()
        .map_err(|e| format!("Error locking receiver to get job: '{}'", e))?;
    let job = guard
        .recv()
        .map_err(|e| format!("Error receiving job for execution: '{}'", e))?;

    trace!("Job received for execution: {}", job);
    let mut implementations = loaded_implementations.try_write()
        .map_err(|_| "Could not gain write access to loaded implementations map")?;
    if implementations.get(&job.implementation_url).is_none() {
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
    }

    let implementation = implementations.get(&job.implementation_url)
        .ok_or("Could not find implementation")?;

    execute_job(job, results_sender, name, implementation)
}

fn execute_job(
    mut job: Job,
    results_sender: &Sender<Job>,
    name: &str,
    implementation: &Arc<dyn Implementation>,
) -> Result<()> {
    trace!("Job #{}: Started executing on '{name}'", job.job_id);
    job.result = implementation.run(&job.input_set);
    trace!("Job #{}: Finished executing on '{name}'", job.job_id);
    results_sender.send(job).chain_err(|| "Error sending job result back after execution")
}

// Load a WASM Implementation from a "file://" Url
fn resolve_implementation(provider: Arc<dyn Provider>,
                          implementation_url: &Url,
) -> Result<Arc<dyn Implementation>> {
    format!("Implementation at '{}' is not loaded", implementation_url);
    // load the supplied implementation for the function from wasm file referenced
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
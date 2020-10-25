use std::sync::{Arc, Mutex};
#[cfg(feature = "debugger")]
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::mpsc;
use std::time::Duration;

use log::{debug, error, info, trace};
use url::Url;

use flowrstructs::manifest::Manifest;
use provider::content::provider::MetaProvider;

#[cfg(feature = "debugger")]
use crate::debug_client::ChannelDebugClient;
#[cfg(feature = "debugger")]
use crate::debug_client::Event as DebugEvent;
#[cfg(feature = "debugger")]
use crate::debug_client::Response as DebugResponse;
#[cfg(feature = "debugger")]
use crate::debugger::Debugger;
use crate::errors::*;
use crate::execution;
use crate::flowruntime;
use crate::loader::Loader;
#[cfg(feature = "metrics")]
use crate::metrics::Metrics;
use crate::run_state::{Job, RunState};
use crate::runtime_client::{ChannelRuntimeClient, Event, Response, RuntimeClient};
use crate::runtime_client::Response::ClientSubmission;

/// A Submission is the struct used to send a flow to the Coordinator for execution. It contains
/// all the information necessary to execute it:
///
/// A new Submission is created supplying:
/// - the manifest of the flow to execute
/// - the maximum number of jobs you want dispatched/executing in parallel
/// - whether to display some execution metrics when the flow completes
/// - an optional DebugClient to allow you to debug the execution
#[derive(PartialEq)]
pub struct Submission {
    manifest_url: Url,
    max_parallel_jobs: usize,
    job_timeout: Duration,
    #[cfg(feature = "debugger")]
    enter_debugger: bool,
}

impl Submission {
    /// Create a new `Submission` of a `Flow` for execution with the specified `Manifest`
    /// of `Functions`, executing it with a maximum of `mac_parallel_jobs` running in parallel
    /// connecting via the optional `DebugClient`
    pub fn new(manifest_url: &Url,
               max_parallel_jobs: usize,
               #[cfg(feature = "debugger")]
               enter_debugger: bool) -> Submission {
        info!("Maximum jobs in parallel limited to {}", max_parallel_jobs);

        Submission {
            manifest_url: manifest_url.clone(),
            max_parallel_jobs,
            job_timeout: Duration::from_secs(60),
            #[cfg(feature = "debugger")]
            enter_debugger,
        }
    }
}

/// The Coordinator is responsible for coordinating the dispatching of jobs (consisting
/// of a set of Input values and an Implementation of a Function) for execution,
/// gathering the resulting Outputs and distributing output values to other connected function's
/// Inputs.
///
/// It accepts Flows to be executed in the form of a Submission struct that has the required
/// information to execute the flow.
pub struct Coordinator {
    /// A channel used to send Jobs out for execution
    job_tx: Sender<Job>,
    /// A channel used to receive Jobs back after execution (now including the job's output)
    job_rx: Receiver<Job>,
    /// A flag that indicates a request to enter the debugger has been made
    #[cfg(feature = "debugger")]
    debug_requested: Arc<AtomicBool>,
    /// Send messages to the client over channels
    runtime_client: Arc<Mutex<ChannelRuntimeClient>>,
    #[cfg(feature = "debugger")]
    /// Send messages to the debug client over channels
    debug_client: Arc<Mutex<ChannelDebugClient>>,
}

/// Create a Submission for a flow to be executed.
/// Instantiate the Coordinator.
/// Send the Submission to the Coordinator to be executed
///
/// # Examples
///
/// ```no_run
/// use std::sync::{Arc, Mutex};
/// use std::io;
/// use std::io::Write;
/// use flowrlib::coordinator::{Coordinator, Submission};
/// use std::process::exit;
/// use flowrstructs::manifest::{Manifest, MetaData};
/// #[cfg(any(feature = "debugger"))]
/// use flowrlib::debug_client::{DebugClient, Response, Param, Event, Response::ExitDebugger};
/// use flowrlib::runtime_client::RuntimeClient;
/// use flowrlib::runtime_client::Response as RuntimeResponse;
/// use flowrlib::runtime_client::Event as RuntimeEvent;
/// use url::Url;
/// use flowrlib::runtime_client::Response::ClientSubmission;
///
/// struct ExampleDebugClient {};
/// #[derive(Debug)]
/// struct ExampleRuntimeClient {};
///
/// impl DebugClient for ExampleDebugClient {
///     fn send_event(&self, event: Event) -> Response {
///         Response::Ack
///     }
/// }
///
/// impl RuntimeClient for ExampleRuntimeClient {
///     fn send_event(&mut self,event: RuntimeEvent) -> RuntimeResponse {
///         RuntimeResponse::Ack
///     }
/// }
///
/// let example_client = ExampleRuntimeClient {};
///
/// let manifest_url = Url::parse("file:///temp/fake.toml").unwrap();
///
/// let mut submission = Submission::new(&manifest_url,
///                                     1 /* num_parallel_jobs */,
///                                     true /* enter debugger on start */);
///
/// let mut coordinator = Coordinator::new( 1 /* num_threads */, );
/// let native = true;
/// coordinator.start(native);
///
/// let (_, client_channel) = coordinator.get_client_channels();
///
/// client_channel.send(ClientSubmission(submission)).unwrap();
///
/// exit(0);
/// ```
///
impl Coordinator {
    /// Create a new `coordinator` with `num_threads` executor threads
    pub fn new(num_threads: usize) -> Self {
        let (job_tx, job_rx, ) = mpsc::channel();
        let (output_tx, output_rx) = mpsc::channel();

        execution::set_panic_hook();

        info!("Starting {} executor threads", num_threads);
        let shared_job_receiver = Arc::new(Mutex::new(job_rx));
        execution::start_executors(num_threads, &shared_job_receiver, &output_tx);

        Coordinator {
            job_tx,
            job_rx: output_rx,
            #[cfg(feature = "debugger")]
            debug_requested: Arc::new(AtomicBool::new(false)),
            runtime_client: Arc::new(Mutex::new(ChannelRuntimeClient::new())),
            #[cfg(feature = "debugger")]
            debug_client: Arc::new(Mutex::new(ChannelDebugClient::new())),
        }
    }

    pub fn get_client_channels(&self) -> (Arc<Mutex<Receiver<Event>>>, Sender<Response>) {
        self.runtime_client.lock().unwrap().get_client_channels()
    }

    #[cfg(feature = "debugger")]
    pub fn get_debug_channels(&self) -> (Arc<Mutex<Receiver<DebugEvent>>>, Sender<DebugResponse>) {
        self.debug_client.lock().unwrap().get_channels()
    }

    fn wait_for_submission(&self) -> Submission {
        loop {
            match self.runtime_client.lock().unwrap().get_response() {
                ClientSubmission(submission) => return submission,
                _ => error!("Was expecting a Submission from the client"),
            }
        }
    }

    /// Start the Coordinator
    pub fn start(&mut self, native: bool) {
        let submission = self.wait_for_submission();

        debug!("Received submission for execution with manifest_url: '{}'", submission.manifest_url.to_string());
        let mut manifest = Self::load_from_manifest(&submission.manifest_url.to_string(),
                                                    self.runtime_client.clone(),
                                                    native).unwrap(); // TODO
        let mut state = RunState::new(manifest.get_functions(), submission.max_parallel_jobs);
        #[cfg(feature = "debugger")]
            let debug_client = ChannelDebugClient::new();
        #[cfg(feature = "debugger")]
            let mut debugger = Debugger::new(&debug_client);
        #[cfg(feature = "metrics")]
            let mut metrics = Metrics::new(state.num_functions());

        // This outer loop is just a way of restarting execution from scratch if the debugger requests it
        'flow_execution: loop {
            debug!("Resetting stats and initializing all functions");
            state.init();
            self.runtime_client.lock().unwrap().send_event(Event::FlowStart);

            #[cfg(feature = "debugger")]
            if submission.enter_debugger {
                debugger.enter(&state);
            }

            #[cfg(feature = "metrics")]
                metrics.reset();

            #[cfg(feature = "debugger")]
                let mut display_next_output;
            let mut restart;

            'inner: loop {
                trace!("{}", state);
                #[cfg(feature = "debugger")]
                if self.debug_requested.load(Ordering::SeqCst) {
                    self.debug_requested.store(false, Ordering::SeqCst); // reset to avoid re-entering
                    debugger.enter(&state);
                }

                let debug_check = self.send_jobs(&mut state,
                                                 #[cfg(feature = "debugger")]
                                                     &mut debugger,
                                                 #[cfg(feature = "metrics")]
                                                     &mut metrics,
                );
                #[cfg(feature = "debugger")]
                    {
                        display_next_output = debug_check.0;
                    }
                restart = debug_check.1;

                // If debugger request it, exit the inner loop which will cause us to reset state
                // and restart execution, in the outer loop
                if restart {
                    break 'inner;
                }

                if state.number_jobs_running() > 0 {
                    match self.job_rx.recv_timeout(submission.job_timeout) {
                        Ok(job) => {
                            #[cfg(feature = "debugger")]
                                {
                                    if display_next_output {
                                        debugger.job_completed(&job);
                                    }
                                }

                            state.complete_job(
                                #[cfg(feature = "metrics")]
                                    &mut metrics,
                                job,
                                #[cfg(feature = "debugger")]
                                    &mut debugger,
                            );
                        }
                        #[cfg(feature = "debugger")]
                        Err(err) => {
                            debugger.panic(&state,
                                           format!("Error in job reception: '{}'", err));
                        }
                        #[cfg(not(feature = "debugger"))]
                        Err(_) => error!("\tError in Job reception")
                    }
                }

                if state.number_jobs_running() == 0 && state.number_jobs_ready() == 0 {
                    // execution is done - but not returning here allows us to go into debugger
                    // at the end of execution, inspect state and possibly reset and rerun
                    break 'inner;
                }
            }

            #[allow(clippy::collapsible_if)]
            if !restart {
                #[cfg(feature = "debugger")]
                    {
                        if submission.enter_debugger {
                            let check = debugger.flow_done(&state);
                            restart = check.1;
                        }
                    }

                if !restart {
                    #[cfg(feature = "metrics")]
                        {
                            metrics.set_jobs_created(state.jobs_created());
                            self.runtime_client.lock().unwrap().send_event(Event::FlowEnd(metrics));
                        }
                    #[cfg(not(feature = "metrics"))]
                        self.runtime_client.lock().unwrap().send_event(Event::FlowEnd);
                    debug!("{}", state);
                    break 'flow_execution;
                }
            }
        }
    }

    fn load_from_manifest(manifest_url: &str, runtime_client: Arc<Mutex<dyn RuntimeClient>>, native: bool) -> Result<Manifest> {
        let mut loader = Loader::new();
        let provider = MetaProvider {};

        // Load this run-time's library of native (statically linked) implementations
        loader.add_lib(&provider,
                       "lib://flowruntime",
                       flowruntime::get_manifest(runtime_client.clone()),
                       "native")
            .chain_err(|| "Could not add 'flowruntime' library to loader")?;

        // If the "native" feature is enabled then load the native flowstdlib if command line arg to do so
        if cfg!(feature = "native") && native {
            loader.add_lib(&provider, "lib://flowstdlib", flowstdlib::get_manifest(), "native")
                .chain_err(|| "Could not add 'flowstdlib' library to loader")?;
        }

        // Load the flow to run from the manifest
        let mut manifest = loader.load_manifest(&provider, manifest_url)
            .chain_err(|| format!("Could not load the flow from manifest: '{}'", manifest_url))?;

        // Find the implementations for all functions in this flow
        loader.resolve_implementations(&mut manifest, manifest_url, &provider).unwrap();

        Ok(manifest)
    }

    /*
        Send as many jobs as possible for parallel execution.
        Return 'true' if the debugger is requesting a restart
    */
    fn send_jobs(&mut self,
                 state: &mut RunState,
                 #[cfg(feature = "debugger")]
                 debugger: &mut Debugger,
                 #[cfg(feature = "metrics")]
                 metrics: &mut Metrics,
    ) -> (bool, bool) {
        let mut display_output = false;
        let mut restart = false;

        while let Some(job) = state.next_job() {
            match self.send_job(job.clone(), state,
                                #[cfg(feature = "debugger")]
                                    debugger,
                                #[cfg(feature = "metrics")]
                                    metrics,
            ) {
                Ok((display, rest)) => {
                    display_output = display;
                    restart = rest;
                }
                Err(err) => {
                    error!("Error sending on 'job_tx': {}", err.to_string());
                    debug!("{}", state);

                    #[cfg(feature = "debugger")]
                        debugger.error(&state, job);
                }
            }
        }

        (display_output, restart)
    }

    /*
        Send a job for execution
    */
    fn send_job(&self,
                job: Job,
                state: &mut RunState,
                #[cfg(feature = "debugger")]
                debugger: &mut Debugger,
                #[cfg(feature = "metrics")]
                metrics: &mut Metrics,
    ) -> Result<(bool, bool)> {
        #[cfg(not(feature = "debugger"))]
            let debug_options = (false, false);

        state.start(&job);
        #[cfg(feature = "metrics")]
            metrics.track_max_jobs(state.number_jobs_running());

        #[cfg(feature = "debugger")]
            let debug_options = debugger.check_prior_to_job(&state, job.job_id, job.function_id);

        let job_id = job.job_id;
        self.job_tx.send(job).chain_err(|| "Sending of job for execution failed")?;
        debug!("Job #{}:\tSent for execution", job_id);

        Ok(debug_options)
    }
}

#[cfg(test)]
mod test {
    use url::Url;

    use crate::coordinator::Submission;
    #[cfg(feature = "debugger")]
    use crate::debug_client::{DebugClient, Event, Response};
    use crate::runtime_client::Event as RuntimeCommand;
    use crate::runtime_client::Response as RuntimeResponse;
    use crate::runtime_client::RuntimeClient;

    #[cfg(feature = "debugger")]
    struct TestDebugClient {}

    #[cfg(feature = "debugger")]
    impl DebugClient for TestDebugClient {
        fn send_event(&self, _event: Event) -> Response {
            Response::Ack
        }
    }

    #[derive(Debug)]
    struct TestRuntimeClient {}

    impl RuntimeClient for TestRuntimeClient {
        fn send_event(&mut self, _command: RuntimeCommand) -> RuntimeResponse {
            RuntimeResponse::Ack
        }
    }

    #[test]
    fn create_submission() {
        let manifest_url = Url::parse("file:///temp/fake/flow.toml").unwrap();
        let _ = Submission::new(&manifest_url, 1,
                                #[cfg(feature = "debugger")]
                                    false,
        );
    }

    #[test]
    fn test_create() {
        let _ = super::Coordinator::new(0);
    }
}
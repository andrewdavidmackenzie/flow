use std::sync::{Arc, Mutex};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::mpsc;
use std::time::Duration;

use log::{debug, error, info, trace};
use url::Url;

use flowrstructs::manifest::Manifest;
use provider::content::provider::MetaProvider;

use crate::client_server::{DebuggerConnection, RuntimeConnection};
use crate::client_server::{ChannelRuntimeClient, RuntimeClient};
#[cfg(feature = "debugger")]
use crate::debugger::Debugger;
use crate::errors::*;
use crate::execution;
use crate::flowruntime;
use crate::loader::Loader;
#[cfg(feature = "metrics")]
use crate::metrics::Metrics;
use crate::run_state::{Job, RunState};
use crate::runtime::{Event, Response};

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
    pub max_parallel_jobs: usize,
    pub job_timeout: Duration,
    #[cfg(feature = "debugger")]
    pub debug: bool,
}

impl Submission {
    /// Create a new `Submission` of a `Flow` for execution with the specified `Manifest`
    /// of `Functions`, executing it with a maximum of `mac_parallel_jobs` running in parallel
    /// connecting via the optional `DebugClient`
    pub fn new(manifest_url: &Url,
               max_parallel_jobs: usize,
               #[cfg(feature = "debugger")]
               debug: bool) -> Submission {
        info!("Maximum jobs in parallel limited to {}", max_parallel_jobs);

        Submission {
            manifest_url: manifest_url.clone(),
            max_parallel_jobs,
            job_timeout: Duration::from_secs(60),
            #[cfg(feature = "debugger")]
            debug,
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
    /// Get messages from clients over channels
    runtime_client: Arc<Mutex<ChannelRuntimeClient>>,
    #[cfg(feature = "debugger")]
    debugger: Debugger,
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
/// use flowrlib::client_server::DebugClient;
/// #[cfg(any(feature = "debugger"))]
/// use flowrlib::debug::{Response, Param, Event, Response::ExitDebugger};
/// use flowrlib::client_server::RuntimeClient;
/// use flowrlib::runtime::Response as RuntimeResponse;
/// use flowrlib::runtime::Event as RuntimeEvent;
/// use url::Url;
/// use flowrlib::runtime::Response::ClientSubmission;
///
/// let manifest_url = Url::parse("file:///temp/fake.toml").unwrap();
///
/// let mut submission = Submission::new(&manifest_url,
///                                     1 /* num_parallel_jobs */,
///                                     true /* enter debugger on start */);
///
/// let (runtime_connection, debugger_connection) = Coordinator::connect(1 /* num_threads */,
///                                                                      true /* native */);
///
/// runtime_connection.client_submit(submission).unwrap();
/// exit(0);
/// ```
///
impl Coordinator {
    /// Create a new `coordinator` with `num_threads` executor threads
    fn new(num_threads: usize) -> Self {
        let (job_tx, job_rx, ) = mpsc::channel();
        let (output_tx, output_rx) = mpsc::channel();

        execution::set_panic_hook();

        info!("Starting {} executor threads", num_threads);
        let shared_job_receiver = Arc::new(Mutex::new(job_rx));
        execution::start_executors(num_threads, &shared_job_receiver, &output_tx);

        Coordinator {
            job_tx,
            job_rx: output_rx,
            runtime_client: Arc::new(Mutex::new(ChannelRuntimeClient::new())),
            #[cfg(feature = "debugger")]
            debugger: Debugger::new(),
        }
    }

    pub fn connect(num_threads: usize, native: bool) -> (RuntimeConnection, DebuggerConnection) {
        let mut coordinator = Coordinator::new(num_threads);

        let runtime_connection = RuntimeConnection::new(&coordinator);
        let debugger_connection = DebuggerConnection::new(&coordinator.debugger);

        std::thread::spawn(move || {
            coordinator.start(native);
        });

        (runtime_connection, debugger_connection)
    }

    pub fn get_client_channels(&self) -> (Arc<Mutex<Receiver<Event>>>, Sender<Response>) {
        self.runtime_client.lock().unwrap().get_client_channels()
    }

    /// Loop waiting for a message from the client.
    /// If the message is a `ClientSubmission` with a submission, then return Some(submission)
    /// If the message is `ClientExiting` then return None
    /// If the message is any other then loop until we find one of the above
    fn wait_for_submission(&self) -> Option<Submission> {
        loop {
            match self.runtime_client.lock() {
                Ok(guard) => {
                    match guard.get_response() {
                        Response::ClientSubmission(submission) => {
                            debug!("Received submission for execution with manifest_url: '{}'", submission.manifest_url.to_string());
                            return Some(submission);
                        }
                        Response::ClientExiting => return None,
                        _ => error!("Was expecting a Submission from the client"),
                    }
                }
                _ => {
                    error!("There was an error accessing the client connection");
                    return None;
                }
            }
        }
    }

    /// Start the Coordinator - this will block the thread it is running on waiting for a submission
    /// It will loop processing submissions until it gets a `ClientExiting` response, then it will also exit
    pub fn start(&mut self, native: bool) {
        while let Some(submission) = self.wait_for_submission() {
            if let Ok(mut manifest) = Self::load_from_manifest(&submission.manifest_url.to_string(),
                                                               self.runtime_client.clone(),
                                                               native) {
                let state = RunState::new(manifest.get_functions(), submission);
                self.execute_flow(state);
            }
        }

        // Exiting
        debug!("Client exiting and no other clients connected, so server is exiting");
    }

    /// Execute a flow by looping while there are jobs to be processed in an inner loop.
    /// There is an outer loop for the case when you are using the debugger, to allow entering
    /// the debugger when the flow ends and at any point resetting all the state and starting
    /// execution again from the initial state
    fn execute_flow(&mut self, mut state: RunState) {
        #[cfg(feature = "metrics")]
            let mut metrics = Metrics::new(state.num_functions());

        // This outer loop is just a way of restarting execution from scratch if the debugger requests it
        'flow_execution: loop {
            state.init();
            #[cfg(feature = "metrics")]
                metrics.reset();

            self.runtime_client.lock().unwrap().send_event(Event::FlowStart);

            #[cfg(feature = "debugger")]
            if state.debug {
                self.debugger.enter(&state);
            }

            #[cfg(feature = "debugger")]
                let mut display_next_output;
            let mut restart;

            'jobs: loop {
                trace!("{}", state);
                #[cfg(feature = "debugger")]
                    self.debugger.check_for_entry(&state);

                let debug_check = self.send_jobs(&mut state,
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
                    break 'jobs;
                }

                if state.number_jobs_running() > 0 {
                    match self.job_rx.recv_timeout(state.job_timeout) {
                        Ok(job) => {
                            #[cfg(feature = "debugger")]
                                {
                                    if display_next_output {
                                        self.debugger.job_completed(&job);
                                    }
                                }

                            state.complete_job(
                                #[cfg(feature = "metrics")]
                                    &mut metrics,
                                job,
                                #[cfg(feature = "debugger")]
                                    &mut self.debugger,
                            );
                        }
                        #[cfg(feature = "debugger")]
                        Err(err) => {
                            if state.debug {
                                self.debugger.panic(&state,
                                                    format!("Error in job reception: '{}'", err));
                            }
                        }
                        #[cfg(not(feature = "debugger"))]
                        Err(_) => error!("\tError in Job reception")
                    }
                }

                if state.number_jobs_running() == 0 && state.number_jobs_ready() == 0 {
                    // execution is done - but not returning here allows us to go into debugger
                    // at the end of execution, inspect state and possibly reset and rerun
                    break 'jobs;
                }
            }

            #[allow(clippy::collapsible_if)]
            if !restart {
                #[cfg(feature = "debugger")]
                    {
                        if state.debug {
                            let check = self.debugger.flow_done(&state);
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
                 #[cfg(feature = "metrics")]
                 metrics: &mut Metrics,
    ) -> (bool, bool) {
        let mut display_output = false;
        let mut restart = false;

        while let Some(job) = state.next_job() {
            match self.send_job(job.clone(), state,
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
                        self.debugger.error(&state, job);
                }
            }
        }

        (display_output, restart)
    }

    /*
        Send a job for execution
    */
    fn send_job(&mut self,
                job: Job,
                state: &mut RunState,
                #[cfg(feature = "metrics")]
                metrics: &mut Metrics,
    ) -> Result<(bool, bool)> {
        #[cfg(not(feature = "debugger"))]
            let debug_options = (false, false);

        state.start(&job);
        #[cfg(feature = "metrics")]
            metrics.track_max_jobs(state.number_jobs_running());

        #[cfg(feature = "debugger")]
            let debug_options = self.debugger.check_prior_to_job(&state, job.job_id, job.function_id);

        let job_id = job.job_id;
        self.job_tx.send(job).chain_err(|| "Sending of job for execution failed")?;
        debug!("Job #{}:\tSent for execution", job_id);

        Ok(debug_options)
    }
}

#[cfg(test)]
mod test {
    use url::Url;

    #[cfg(feature = "debugger")]
    use crate::client_server::DebugClient;
    use crate::client_server::RuntimeClient;
    use crate::coordinator::Submission;
    #[cfg(feature = "debugger")]
    use crate::debug::Event as DebugEvent;
    use crate::runtime::Event as RuntimeCommand;
    use crate::runtime::Response as RuntimeResponse;

    #[cfg(feature = "debugger")]
    struct TestDebugClient {}

    #[cfg(feature = "debugger")]
    impl DebugClient for TestDebugClient {
        fn send_event(&self, _event: DebugEvent) {}
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
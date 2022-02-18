use std::sync::{Arc, Mutex};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::time::Duration;

use log::{debug, error, info, trace};
use serde_derive::{Deserialize, Serialize};
use simpath::Simpath;
use url::Url;

use flowcore::lib_provider::{MetaProvider, Provider};

use crate::client_provider::ClientProvider;
use crate::client_server::{DONT_WAIT, ServerConnection, WAIT};
use crate::context;
#[cfg(feature = "debugger")]
use crate::debugger::Debugger;
use crate::errors::*;
use crate::execution;
use crate::job::Job;
use crate::loader::Loader;
#[cfg(feature = "metrics")]
use crate::metrics::Metrics;
use crate::run_state::RunState;
use crate::runtime_messages::{ClientMessage, ServerMessage};

/// `RUNTIME_SERVICE_NAME` is the name of the runtime services and can be used to discover it by name
pub const RUNTIME_SERVICE_NAME: &str = "runtime._flowr._tcp.local";
/// `DEBUG_SERVICE_NAME` is the name of the runtime services and can be used to discover it by name
#[cfg(feature = "debugger")]
pub const DEBUG_SERVICE_NAME: &str = "debug._flowr._tcp.local";

/// The `Coordinator` of flow execution can run in one of these three modes:
/// - `ClientOnly`      - only as a client to submit flows for execution to a server
/// - `ServerOnly`      - only as a server waiting for submissions for execution from a client
/// - `ClientAndServer` - as both Client and Server, in separate threads
#[derive(PartialEq, Clone, Debug)]
pub enum Mode {
    /// `Coordinator` mode where it runs as just a client for a server running in another process
    ClientOnly,
    /// `Coordinator` mode where it runs as just a server, clients must run in another process
    ServerOnly,
    /// `Coordinator` mode where a single process runs as a client and s server in different threads
    ClientAndServer,
}

/// A `Submission` is the struct used to send a flow to the Coordinator for execution. It contains
/// all the information necessary to execute it:
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Submission {
    /// The URL where the manifest of the flow to execute can be found
    manifest_url: Url,
    /// The maximum number of jobs you want dispatched/executing in parallel
    pub max_parallel_jobs: usize,
    /// The Duration to wait before timing out when waiting for jobs to complete
    pub job_timeout: Duration,
    /// Whether to debug the flow while executing it
    #[cfg(feature = "debugger")]
    pub debug: bool,
}

impl Submission {
    /// Create a new `Submission` of a `Flow` for execution with the specified `Manifest`
    /// of `Functions`, executing it with a maximum of `mac_parallel_jobs` running in parallel
    /// connecting via the optional `DebugClient`
    pub fn new(
        manifest_url: &Url,
        max_parallel_jobs: usize,
        #[cfg(feature = "debugger")] debug: bool,
    ) -> Submission {
        info!("Maximum jobs in parallel limited to {}", max_parallel_jobs);

        Submission {
            manifest_url: manifest_url.to_owned(),
            max_parallel_jobs,
            job_timeout: Duration::from_secs(60),
            #[cfg(feature = "debugger")]
            debug,
        }
    }
}

/// The `Coordinator` is responsible for coordinating the dispatching of jobs (consisting
/// of a set of Input values and an Implementation of a Function) for execution,
/// gathering the resulting Outputs and distributing output values to other connected function's
/// Inputs.
///
/// It accepts Flows to be executed in the form of a `Submission` struct that has the required
/// information to execute the flow.
///
/// # Example Submission of a flow for execution to the Coordinator
///
/// Instantiate the Coordinator server that receives the submitted flows to be executed, specifying
/// Create a `Submission` for the flow to be executed.
/// Create a `ClientConnection` to the `Coordinator` server
/// Send a `Submission` to the Coordinator to be executed
///
/// ```no_run
/// use std::sync::{Arc, Mutex};
/// use std::io;
/// use std::io::Write;
/// use flowrlib::coordinator::{Coordinator, Submission, Mode, RUNTIME_SERVICE_NAME, DEBUG_SERVICE_NAME};
/// use std::process::exit;
/// use flowcore::model::flow_manifest::FlowManifest;
/// use flowcore::model::metadata::MetaData;
/// use flowrlib::runtime_messages::ClientMessage::ClientSubmission;
/// use simpath::Simpath;
/// use url::Url;
/// use flowrlib::client_server::{ClientConnection, ServerConnection, ServerInfo, Method};
/// use flowrlib::runtime_messages::{ServerMessage, ClientMessage};
///
/// let runtime_server_connection = ServerConnection::new(RUNTIME_SERVICE_NAME, Method::Tcp(None)).unwrap();
/// let debug_server_connection = ServerConnection::new(DEBUG_SERVICE_NAME, Method::Tcp(None)).unwrap();
/// let mut runtime_server_info = runtime_server_connection.get_server_info().clone();///
///
/// std::thread::spawn(move || {
///         let _ = Coordinator::start(
///         1,
///         Simpath::new("fake path"),
///         true,
///         runtime_server_connection,
///         #[cfg(feature = "debugger")] debug_server_connection,
///     );
/// });
///
/// let mut submission = Submission::new(&Url::parse("file:///temp/fake.toml").unwrap(),
///                                     1 /* num_parallel_jobs */,
///                                     true /* enter debugger on start */);
/// let runtime_client_connection = ClientConnection::new(&mut runtime_server_info).unwrap();
/// runtime_client_connection.send(ClientSubmission(submission)).unwrap();
/// exit(0);
/// ```
pub struct Coordinator {
    /// A channel used to send Jobs out for execution
    job_tx: Sender<Job>,
    /// A channel used to receive Jobs back after execution (now including the job's output)
    job_rx: Receiver<Job>,
    /// Get and Send messages to/from the runtime client
    runtime_server_connection: Arc<Mutex<ServerConnection>>,
    #[cfg(feature = "debugger")]
    debugger: Debugger,
}

impl Coordinator {
    // Create a new `coordinator` with `num_threads` executor threads
    fn new(
        runtime_server_connection: ServerConnection,
        #[cfg(feature = "debugger")] debug_server_connection: ServerConnection,
        num_threads: usize,
    ) -> Self {
        let (job_tx, job_rx) = mpsc::channel();
        let (output_tx, output_rx) = mpsc::channel();

        execution::set_panic_hook();

        info!("Starting {} executor threads", num_threads);
        let shared_job_receiver = Arc::new(Mutex::new(job_rx));
        execution::start_executors(num_threads, &shared_job_receiver, &output_tx);

        Coordinator {
            job_tx,
            job_rx: output_rx,
            runtime_server_connection: Arc::new(Mutex::new(runtime_server_connection)),
            #[cfg(feature = "debugger")]
            debugger: Debugger::new(debug_server_connection),
        }
    }

    /// Create a new `Coordinator` and then enter it's `submission_loop()` accepting and executing
    /// flows submitted for execution.
    pub fn start(
        num_threads: usize,
        lib_search_path: Simpath,
        native: bool,
        runtime_server_connection: ServerConnection,
        #[cfg(feature = "debugger")] debug_server_connection: ServerConnection,
    ) -> Result<()> {
        let mut coordinator = Coordinator::new(
            runtime_server_connection,
            #[cfg(feature = "debugger")]
            debug_server_connection,
            num_threads,
        );

        coordinator.submission_loop(
            lib_search_path,
            native,
        )
    }

    /*
       Enter the Coordinator's Submission Loop - this will block the thread it is running on and
       wait for a submission to be sent from a client
       It will loop receiving and processing submissions until it gets a `ClientExiting` response,
       then it will also exit
    */
    fn submission_loop(
        &mut self,
        lib_search_path: Simpath,
        native: bool,
    ) -> Result<()> {
        let mut loader = Loader::new();
        let server_provider = MetaProvider::new(lib_search_path);
        let client_provider = ClientProvider::new(self.runtime_server_connection.clone());
        Self::load_native_libs(
            &mut loader,
            &server_provider,
            self.runtime_server_connection.clone(),
            native,
        )?;

        while let Some(submission) = self.wait_for_submission()? {
            match loader.load_flow(&server_provider, &client_provider, &submission.manifest_url) {
                Ok(mut manifest) => {
                    let state = RunState::new(manifest.get_functions(), submission);
                    if self.execute_flow(state)? {
                        break;
                    }
                }
                Err(e) => error!(
                    "Could not load the flow from manifest url: '{}'\n    {}",
                    submission.manifest_url, e
                ),
            }
        }

        debug!("Server has exited submission loop and will close connection");
        self.close_connection()
    }

    fn close_connection(&mut self) -> Result<()> {
        let mut connection = self
            .runtime_server_connection
            .lock()
            .map_err(|e| format!("Could not lock Server Connection: {}", e))?;

        connection.send(ServerMessage::ServerExiting)
    }

    // Loop waiting for one of the following two messages from the client:
    //  - `ClientSubmission` with a submission, then return Ok(Some(submission))
    //  - `ClientExiting` then return Ok(None)
    fn wait_for_submission(&mut self) -> Result<Option<Submission>> {
        loop {
            info!("'flowr' server is waiting to receive a 'Submission'");
            match self.runtime_server_connection.lock() {
                Ok(guard) => match guard.receive(WAIT) {
                    Ok(ClientMessage::ClientSubmission(submission)) => {
                        debug!(
                            "Server received a submission for execution with manifest_url: '{}'",
                            submission.manifest_url
                        );
                        return Ok(Some(submission));
                    }
                    Ok(ClientMessage::ClientExiting) => return Ok(None),
                    Ok(r) => error!("Server did not expect response from client: '{:?}'", r),
                    Err(e) => bail!("Server error while waiting for submission: '{}'", e),
                },
                _ => {
                    error!("Server could not lock context");
                    return Ok(None);
                }
            }
        }
    }

    //noinspection RsReassignImmutable
    // Execute a flow by looping while there are jobs to be processed in an inner loop.
    // There is an outer loop for the case when you are using the debugger, to allow entering
    // the debugger when the flow ends and at any point resetting all the state and starting
    // execution again from the initial state
    fn execute_flow(&mut self, mut state: RunState) -> Result<bool> {
        #[cfg(feature = "metrics")]
        let mut metrics = Metrics::new(state.num_functions());

        #[cfg(feature = "debugger")]
        if state.debug {
            self.debugger.start();
        }

        // This outer loop is just a way of restarting execution from scratch if the debugger requests it
        'flow_execution: loop {
            state.init();
            #[cfg(feature = "metrics")]
            metrics.reset();

            #[cfg(feature = "debugger")]
            let mut display_next_output;
            #[cfg(feature = "debugger")]
            let mut restart: bool;
            #[cfg(not(feature = "debugger"))]
            let restart: bool = false;

            // If debugging then check if we should enter the debugger
            #[cfg(feature = "debugger")]
            if state.debug {
                let debug_check = self.debugger.wait_for_command(&state);
                if debug_check.2 {
                    return Ok(true); // User requested via debugger to exit execution
                }
            }

            self.runtime_server_connection
                .lock()
                .map_err(|_| "Could not lock server context")?
                .send_and_receive_response::<ServerMessage, ClientMessage>(ServerMessage::FlowStart)?;

            'jobs: loop {
                trace!("{}", state);
                #[cfg(feature = "debugger")]
                if state.debug && self.should_enter_debugger()? {
                    let debug_check = self.debugger.wait_for_command(&state);
                    if debug_check.2 {
                        return Ok(true); // User requested via debugger to exit execution
                    }
                }

                let _debug_check = self.send_jobs(
                    &mut state,
                    #[cfg(feature = "metrics")]
                    &mut metrics,
                );

                #[cfg(feature = "debugger")]
                if _debug_check.2 {
                    return Ok(true); // User requested via debugger to exit execution
                }

                #[cfg(feature = "debugger")]
                {
                    display_next_output = _debug_check.0;
                    restart = _debug_check.1;

                    // If debugger request it, exit the inner job loop which will cause us to reset state
                    // and restart execution, in the outer flow_execution loop
                    if restart {
                        break 'jobs;
                    }
                }

                if state.number_jobs_running() > 0 {
                    match self.job_rx.recv_timeout(state.job_timeout) {
                        Ok(job) => {
                            #[cfg(feature = "debugger")]
                            if job.result.is_err() {
                                if state.debug {
                                    let _ = self.debugger.job_error(&state,&job);
                                }
                            } else {
                                if display_next_output {
                                    self.debugger.job_completed(&job);
                                }

                                state.complete_job(
                                    #[cfg(feature = "metrics")]
                                        &mut metrics,
                                    &job,
                                    #[cfg(feature = "debugger")]
                                        &mut self.debugger,
                                );
                            }
                        }

                        #[cfg(feature = "debugger")]
                        Err(err) => {
                            if state.debug {
                                self.debugger
                                    .panic(&state, format!("Error in job reception: '{}'", err));
                            }
                        }
                        #[cfg(not(feature = "debugger"))]
                        Err(e) => error!("\tError in Job reception: {}", e),
                    }
                }

                if state.number_jobs_running() == 0 && state.number_jobs_ready() == 0 {
                    // execution is done - but not returning here allows us to go into debugger
                    // at the end of execution, inspect state and possibly reset and rerun
                    break 'jobs;
                }
            } // 'jobs loop end

            // flow execution has ended
            #[allow(clippy::collapsible_if)]
            if !restart {
                #[cfg(feature = "debugger")]
                {
                    // If debugging then enter the debugger for a final time before ending flow execution
                    if state.debug {
                        let debug_check = self.debugger.execution_ended(&state);
                        if debug_check.2 {
                            return Ok(true); // Exit debugger
                        }

                        restart = debug_check.1;
                    }
                }

                // if the debugger has not requested a restart of the flow
                if !restart {
                    self.end_flow(
                        #[cfg(feature = "metrics")]
                        &state,
                        #[cfg(feature = "metrics")]
                        metrics,
                    )?;
                    debug!("{}", state);
                    break 'flow_execution;
                }
            }
        }

        Ok(false)
    }

    #[cfg(feature = "metrics")]
    fn end_flow(&mut self, state: &RunState, mut metrics: Metrics) -> Result<()> {
        metrics.set_jobs_created(state.jobs_created());
        self.runtime_server_connection
            .lock()
            .map_err(|_| "Could not lock server context")?
            .send(ServerMessage::FlowEnd(metrics))
    }

    #[cfg(not(feature = "metrics"))]
    fn end_flow(&mut self) -> Result<()> {
        self.runtime_server_connection
            .lock()
            .map_err(|_| "Could not lock server context")?
            .send(ServerMessage::FlowEnd)
    }

    /*
       See if the runtime client has sent a message to request us to enter the debugger,
       if so, return Ok(true).
       A different message or Absence of a message returns Ok(false)
    */
    #[cfg(feature = "debugger")]
    fn should_enter_debugger(&mut self) -> Result<bool> {
        let msg = self
            .runtime_server_connection
            .lock()
            .map_err(|_| "Could not lock server context")?
            .receive(DONT_WAIT);
        match msg {
            Ok(ClientMessage::EnterDebugger) => {
                debug!("Got EnterDebugger message");
                Ok(true)
            }
            Ok(m) => {
                debug!("Got {:?} message", m);
                Ok(false)
            }
            _ => Ok(false),
        }
    }

    fn load_native_libs(
        loader: &mut Loader,
        provider: &dyn Provider,
        server_connection: Arc<Mutex<ServerConnection>>,
        native: bool,
    ) -> Result<()> {
        let flowruntimelib_url =
            Url::parse("lib://context").chain_err(|| "Could not parse context lib url")?;

        // Load this run-time's library of native (statically linked) implementations
        loader
            .add_lib(
                provider,
                context::get_manifest(server_connection)?,
                &flowruntimelib_url,
            )
            .chain_err(|| "Could not add 'context' library to loader")?;

        // If the "flowstdlib" optional dependency is used and the command line options request
        // a native implementation of libs, then load the native version of it
        #[cfg(feature = "flowstdlib")]
        if native {
            let flowstdlib_url = Url::parse("lib://flowstdlib")
                .chain_err(|| "Could not parse flowstdlib lib url")?;
            loader
                .add_lib(
                    provider,
                    flowstdlib::manifest::get_manifest().chain_err(|| "Could not get flowstdlib manifest")?,
                    &flowstdlib_url,
                )
                .chain_err(|| "Could not add 'flowstdlib' library to loader")?;
        }

        Ok(())
    }

    // Send as many jobs as possible for parallel execution.
    // Return 'true' if the debugger is requesting a restart
    fn send_jobs(
        &mut self,
        state: &mut RunState,
        #[cfg(feature = "metrics")] metrics: &mut Metrics,
    ) -> (bool, bool, bool) {
        let mut display_output = false;
        let mut restart = false;
        let mut abort = false;

        while let Some(job) = state.next_job() {
            match self.send_job(
                &job,
                state,
                #[cfg(feature = "metrics")]
                metrics,
            ) {
                Ok((display, rest, leave)) => {
                    display_output = display;
                    restart = rest;
                    abort = leave;
                }
                Err(err) => {
                    error!("Error sending on 'job_tx': {}", err.to_string());
                    debug!("{}", state);

                    #[cfg(feature = "debugger")]
                    self.debugger.job_error(state, &job);
                }
            }
        }

        (display_output, restart, abort)
    }

    // Send a job for execution
    fn send_job(
        &mut self,
        job: &Job,
        state: &mut RunState,
        #[cfg(feature = "metrics")] metrics: &mut Metrics,
    ) -> Result<(bool, bool, bool)> {
        #[cfg(not(feature = "debugger"))]
        let debug_options = (false, false, false);

        state.start(job);
        #[cfg(feature = "metrics")]
        metrics.track_max_jobs(state.number_jobs_running());

        #[cfg(feature = "debugger")]
        let debug_options = self
            .debugger
            .check_prior_to_job(state, job.job_id, job.function_id);

        // Jobs maybe sent to remote nodes over network so have to be self--contained - clone OK
        self.job_tx
            .send(job.clone())
            .chain_err(|| "Sending of job for execution failed")?;
        debug!("Job #{}:\tSent for execution", job.job_id);

        Ok(debug_options)
    }
}

#[cfg(test)]
mod test {
    use url::Url;

    use crate::coordinator::Submission;

    #[test]
    fn create_submission() {
        let manifest_url = Url::parse("file:///temp/fake/flow.toml").expect("Could not create Url");
        let _ = Submission::new(
            &manifest_url,
            1,
            #[cfg(feature = "debugger")]
            false,
        );
    }
}

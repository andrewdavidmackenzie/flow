use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use log::{debug, error, info, trace};
use serde_derive::{Deserialize, Serialize};
use simpath::Simpath;
use url::Url;

use flowcore::flow_manifest::FlowManifest;
use flowcore::lib_provider::{LibProvider, MetaProvider};

#[cfg(feature = "debugger")]
use crate::client_server::{DebugClientConnection, DebugServerConnection};
use crate::client_server::{RuntimeClientConnection, RuntimeServerConnection};
#[cfg(feature = "debugger")]
use crate::debugger::Debugger;
use crate::errors::*;
use crate::execution;
use crate::flowruntime;
use crate::loader::Loader;
#[cfg(feature = "metrics")]
use crate::metrics::Metrics;
use crate::run_state::{Job, RunState};
use crate::runtime_messages::{ClientMessage, ServerMessage};

/// Coordinator and hence the overall `flowr` process can run in one of these three modes:
/// - Client - this only acts as a client to submit flows for execution to a server
/// - Server - run as a server waiting for submissions for execution from a client
/// - ClientAndServer - this process does both, running client and server in separate threads
#[derive(PartialEq, Clone, Debug)]
pub enum Mode {
    /// `flowr` mode where it runs as just a client for a server running in another process
    Client,
    /// `flowr` mode where it runs as just a server, clients must run in another process
    Server,
    /// `flowr` mode where a single process runs as a client and s server in different threads
    ClientAndServer,
}

/// A Submission is the struct used to send a flow to the Coordinator for execution. It contains
/// all the information necessary to execute it:
#[derive(Serialize, Deserialize, PartialEq, Debug)]
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
    /// Get and Send messages to/from the runtime client
    runtime_server_context: Arc<Mutex<RuntimeServerConnection>>,
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
/// use flowrlib::coordinator::{Coordinator, Submission, Mode};
/// use std::process::exit;
/// use flowcore::flow_manifest::{FlowManifest, MetaData};
/// use flowrlib::runtime_messages::ClientMessage as RuntimeResponse;
/// use flowrlib::runtime_messages::ServerMessage as RuntimeEvent;
/// use flowrlib::runtime_messages::ClientMessage::ClientSubmission;
/// use simpath::Simpath;
/// use url::Url;
///
/// let (runtime_client_connection, control_c_connection, debug_client_connection) = Coordinator::server(1 /* num_threads */,
///                                                                     Simpath::new("fake path"),
///                                                                     true,  /* native */
///                                                                     Mode::ClientAndServer,
///                                                                     None   /* server hostname */)
///                                                 .unwrap();
///
/// let mut submission = Submission::new(&Url::parse("file:///temp/fake.toml").unwrap(),
///                                     1 /* num_parallel_jobs */,
///                                     true /* enter debugger on start */);
///
/// runtime_client_connection.client_send(ClientSubmission(submission)).unwrap();
/// exit(0);
/// ```
///
impl Coordinator {
    /// Create a new `coordinator` with `num_threads` executor threads
    fn new(
        runtime_server_context: RuntimeServerConnection,
        #[cfg(feature = "debugger")] debug_server_context: DebugServerConnection,
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
            runtime_server_context: Arc::new(Mutex::new(runtime_server_context)),
            #[cfg(feature = "debugger")]
            debugger: Debugger::new(debug_server_context),
        }
    }

    /// Start the Coordinator in the appropriate `Mode`
    #[cfg(feature = "debugger")]
    pub fn server(
        num_threads: usize,
        lib_search_path: Simpath,
        native: bool,
        mode: Mode,
        server_hostname: Option<&str>,
    ) -> Result<(
        RuntimeClientConnection,
        RuntimeClientConnection,
        DebugClientConnection,
    )> {
        let runtime_server_context = RuntimeServerConnection::new(server_hostname);
        let debug_server_context = DebugServerConnection::new(server_hostname);

        let runtime_client_connection = RuntimeClientConnection::new(&runtime_server_context);
        let control_c_connection = RuntimeClientConnection::new(&runtime_server_context);
        let debug_client_connection = DebugClientConnection::new(&debug_server_context);

        if mode != Mode::Client {
            let mut coordinator =
                Coordinator::new(runtime_server_context, debug_server_context, num_threads);

            if mode == Mode::Server {
                info!("Starting 'flowr' server process");
                coordinator.submission_loop(lib_search_path, native, mode)?;
                info!("'flowr' server process has exited");
            } else {
                std::thread::spawn(move || {
                    info!("Starting 'flowr' server thread");
                    let _ = coordinator.submission_loop(lib_search_path, native, mode);
                    info!("'flowr' server thread has exited");
                });
            }
        }

        Ok((
            runtime_client_connection,
            control_c_connection,
            debug_client_connection,
        ))
    }

    #[cfg(not(feature = "debugger"))]
    pub fn server(
        num_threads: usize,
        lib_search_path: Simpath,
        native: bool,
        mode: Mode,
        server_hostname: Option<&str>,
    ) -> Result<RuntimeClientConnection> {
        let runtime_server_context = RuntimeServerConnection::new(server_hostname);
        let runtime_client_connection = RuntimeClientConnection::new(&runtime_server_context);

        if mode != Mode::Client {
            let mut coordinator = Coordinator::new(runtime_server_context, num_threads);

            if mode == Mode::Server {
                info!("Starting 'flowr' server on main thread");
                coordinator.submission_loop(lib_search_path, native, server_only)?;
            } else {
                std::thread::spawn(move || {
                    info!("Starting 'flowr' server as background thread");
                    if let Err(e) =
                        coordinator.submission_loop(lib_search_path, native, server_only)
                    {
                        error!("Error starting Coordinator in background thread: '{}'", e);
                    }
                });
            }
        }

        Ok(runtime_client_connection)
    }

    /// Enter the Coordinator's Submission Loop - this will block the thread it is running on and
    /// wait for a submission to be sent from a client
    /// It will loop receiving and processing submissions until it gets a `ClientExiting` response,
    /// then it will also exit
    pub fn submission_loop(
        &mut self,
        lib_search_path: Simpath,
        native: bool,
        mode: Mode,
    ) -> Result<()> {
        let mut loader = Loader::new();
        let provider = MetaProvider::new(lib_search_path);

        while let Some(submission) = self.wait_for_submission()? {
            match Self::load_from_manifest(
                &submission.manifest_url,
                &mut loader,
                &provider,
                self.runtime_server_context.clone(),
                native,
            ) {
                Ok(mut manifest) => {
                    let state = RunState::new(manifest.get_functions(), submission);
                    if self.execute_flow(state)? {
                        break;
                    }
                }
                Err(e) if mode == Mode::Server => {
                    error!(
                        "Error in server process submission loop, waiting for new submissions. {}",
                        e
                    )
                }
                Err(e) => {
                    error!("{}", e);
                    error!("Error in server thread, exiting.");
                    break;
                }
            }
        }

        debug!("Server has exited submission loop and will close connection");
        self.close_connection()?;

        Ok(())
    }

    fn close_connection(&mut self) -> Result<()> {
        let mut connection = self
            .runtime_server_context
            .lock()
            .map_err(|e| format!("Could not lock Server Connection: {}", e))?;

        connection.send_message_only(ServerMessage::ServerExiting)?;
        connection.close()
    }

    // Loop waiting for a message from the client.
    // If the message is a `ClientSubmission` with a submission, then return Some(submission)
    // If the message is `ClientExiting` then return None
    // If the message is any other then loop until we find one of the above
    fn wait_for_submission(&mut self) -> Result<Option<Submission>> {
        self.runtime_server_context
            .lock()
            .map_err(|e| format!("Could not lock Server Connection: {}", e))?
            .start()?;
        #[cfg(feature = "debugger")]
        self.debugger.start();

        loop {
            info!("'flowr' server is waiting to receive a 'Submission'");
            match self.runtime_server_context.lock() {
                Ok(guard) => match guard.get_message() {
                    Ok(ClientMessage::ClientSubmission(submission)) => {
                        debug!(
                            "Server received a submission for execution with manifest_url: '{}'",
                            submission.manifest_url
                        );
                        return Ok(Some(submission));
                    }
                    Ok(ClientMessage::ClientExiting) => return Ok(None),
                    Ok(r) => error!("Server did not expect response from client: '{:?}'", r),
                    Err(e) => error!("Server error while waiting for submission: '{}'", e),
                },
                _ => {
                    error!("Server could not lock context");
                    return Ok(None);
                }
            }
        }
    }

    //noinspection RsTypeCheck
    // Execute a flow by looping while there are jobs to be processed in an inner loop.
    // There is an outer loop for the case when you are using the debugger, to allow entering
    // the debugger when the flow ends and at any point resetting all the state and starting
    // execution again from the initial state
    fn execute_flow(&mut self, mut state: RunState) -> Result<bool> {
        #[cfg(feature = "metrics")]
        let mut metrics = Metrics::new(state.num_functions());

        // This outer loop is just a way of restarting execution from scratch if the debugger requests it
        'flow_execution: loop {
            state.init();
            #[cfg(feature = "metrics")]
            metrics.reset();

            self.runtime_server_context
                .lock()
                .map_err(|_| "Could not lock server context")?
                .send_message(ServerMessage::FlowStart)?;

            #[cfg(feature = "debugger")]
            let mut display_next_output;
            let mut restart;

            // If debugging then check if we should enter the debugger
            #[cfg(feature = "debugger")]
            if state.debug {
                let debug_check = self.debugger.enter(&state);
                if debug_check.2 {
                    return Ok(true); // User requested via debugger to exit execution
                }
            }

            'jobs: loop {
                trace!("{}", state);
                #[cfg(feature = "debugger")]
                if state.debug && self.should_enter_debugger()? {
                    let debug_check = self.debugger.enter(&state);
                    if debug_check.2 {
                        return Ok(true); // User requested via debugger to exit execution
                    }
                }

                let debug_check = self.send_jobs(
                    &mut state,
                    #[cfg(feature = "metrics")]
                    &mut metrics,
                );

                #[cfg(feature = "debugger")]
                if debug_check.2 {
                    return Ok(true); // User requested via debugger to exit execution
                }

                #[cfg(feature = "debugger")]
                {
                    display_next_output = debug_check.0;
                    restart = debug_check.1;

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
                        let debug_check = self.debugger.flow_done(&state);
                        if debug_check.2 {
                            return Ok(true); // Exit debugger
                        }

                        restart = debug_check.1;
                    }
                }

                // if the debugger has not requested a restart of the flow
                if !restart {
                    #[cfg(feature = "metrics")]
                    self.end_flow(&state, metrics)?;
                    #[cfg(not(feature = "metrics"))]
                    self.end_flow()?;
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
        self.runtime_server_context
            .lock()
            .map_err(|_| "Could not lock server context")?
            .send_message_only(ServerMessage::FlowEnd(metrics))
    }

    #[cfg(not(feature = "metrics"))]
    fn end_flow(&mut self) -> Result<()> {
        self.runtime_server_context
            .lock()
            .map_err(|_| "Could not lock server context")?
            .send_message_only(ServerMessage::FlowEnd)
    }

    /* TODO - this is not working yet :-(

       See if the runtime client has sent us a message to request us to enter the debugger.

       Absence of a message is returned as an Error.
    */
    #[cfg(feature = "debugger")]
    fn should_enter_debugger(&mut self) -> Result<bool> {
        let msg = self
            .runtime_server_context
            .lock()
            .map_err(|_| "Could not lock server context")?
            .get_message();
        match msg {
            Ok(ClientMessage::EnterDebugger) => {
                debug!("Got enter debugger message");
                Ok(true)
            }
            Ok(m) => {
                debug!("Got {:?} message", m);
                Ok(false)
            }
            _ => Ok(false),
        }
    }

    fn load_from_manifest(
        manifest_url: &Url,
        loader: &mut Loader,
        provider: &dyn LibProvider,
        server_context: Arc<Mutex<RuntimeServerConnection>>,
        native: bool,
    ) -> Result<FlowManifest> {
        let flowruntimelib_url =
            Url::parse("lib://flowruntime").chain_err(|| "Could not parse flowruntime lib url")?;

        // Load this run-time's library of native (statically linked) implementations
        loader
            .add_lib(
                provider,
                flowruntime::get_manifest(server_context)?,
                &flowruntimelib_url,
            )
            .chain_err(|| "Could not add 'flowruntime' library to loader")?;

        // If the "native" feature is enabled and command line options request it
        // then load the native version of flowstdlib
        if cfg!(feature = "native") && native {
            let flowstdlib_url = Url::parse("lib://flowstdlib")
                .chain_err(|| "Could not parse flowstdlib lib url")?;
            loader
                .add_lib(
                    provider,
                    flowstdlib::get_manifest().chain_err(|| "Could not get flowstdlib manifest")?,
                    &flowstdlib_url,
                )
                .chain_err(|| "Could not add 'flowstdlib' library to loader")?;
        }

        // Load the flow to run from the manifest
        let manifest = loader
            .load_flow_manifest(provider, manifest_url)
            .chain_err(|| {
                format!(
                    "Could not load the flow from manifest url: '{}'",
                    manifest_url
                )
            })?;

        Ok(manifest)
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
                    self.debugger.job_error(state, job);
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
        let debug_options = (false, false);

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

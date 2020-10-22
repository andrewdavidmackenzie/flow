use std::sync::{Arc, Mutex};
#[cfg(feature = "debugger")]
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, SendError};
use std::sync::mpsc;
use std::time::Duration;

use log::{debug, error, info, trace};

#[cfg(feature = "debugger")]
use crate::debug_client::DebugClient;
#[cfg(feature = "debugger")]
use crate::debugger::Debugger;
use crate::execution;
use crate::manifest::Manifest;
use crate::manifest::MetaData;
#[cfg(feature = "metrics")]
use crate::metrics::Metrics;
use crate::run_state::Job;
use crate::run_state::RunState;
use crate::runtime_client::{Event, RuntimeClient};

/// A Submission is the struct used to send a flow to the Coordinator for execution. It contains
/// all the information necessary to execute it:
///
/// A new Submission is created supplying:
/// - the manifest of the flow to execute
/// - the maximum number of jobs you want dispatched/executing in parallel
/// - whether to display some execution metrics when the flow completes
/// - an optional DebugClient to allow you to debug the execution
///
/// let mut submission = Submission::new(manifest,
///                                     1 /* num_parallel_jobs */,
///                                     false /* display_metrics */,
///                                     None /* debug client */);
pub struct Submission<'a> {
    _metadata: MetaData,
    display_metrics: bool,
    #[cfg(feature = "metrics")]
    metrics: Metrics,
    runtime_client: Arc<Mutex<dyn RuntimeClient>>,
    job_timeout: Duration,
    state: RunState,
    #[cfg(feature = "debugger")]
    debugger: Debugger<'a>,
    #[cfg(feature = "debugger")]
    enter_debugger: bool,
}

impl<'a> Submission<'a> {
    /// Create a new `Submission` of a `Flow` for execution with the specified `Manifest`
    /// of `Functions`, executing it with a maximum of `mac_parallel_jobs` running in parallel
    /// connecting via the optional `DebugClient`
    pub fn new(mut manifest: Manifest,
               max_parallel_jobs: usize,
               display_metrics: bool,
               runtime_client: Arc<Mutex<dyn RuntimeClient>>,
               #[cfg(feature = "debugger")]
               client: &'a dyn DebugClient,
               #[cfg(feature = "debugger")]
               enter_debugger: bool) -> Submission<'a> {
        info!("Maximum jobs in parallel limited to {}", max_parallel_jobs);
        let output_timeout = Duration::from_secs(60);

        let state = RunState::new(manifest.get_functions(), max_parallel_jobs);

        #[cfg(feature = "metrics")]
            let metrics = Metrics::new(state.num_functions());

        Submission {
            _metadata: manifest.get_metadata().clone(),
            display_metrics,
            #[cfg(feature = "metrics")]
            metrics,
            runtime_client,
            job_timeout: output_timeout,
            state,
            #[cfg(feature = "debugger")]
            debugger: Debugger::new(client),
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
/// use flowrlib::manifest::{Manifest, MetaData};
/// #[cfg(any(feature = "debugger"))]
/// use flowrlib::debug_client::{DebugClient, Response, Param, Event, Response::ExitDebugger};
/// use flowrlib::runtime_client::RuntimeClient;
/// use flowrlib::runtime_client::Response as RuntimeResponse;
/// use flowrlib::runtime_client::Event as RuntimeCommand;
///
/// struct ExampleDebugClient {};
/// #[derive(Debug)]
/// struct ExampleRuntimeClient {};
///
/// let meta_data = MetaData {
///                     name: "test".into(),
///                     description: "Test submission".into(),
///                     version: "0.0.1".into(),
///                     authors: vec!("test user".to_string())
///                 };
///
/// let manifest = Manifest::new(meta_data);
///
/// impl DebugClient for ExampleDebugClient {
///     fn send_event(&self, event: Event) -> Response {
///         Response::Ack
///     }
/// }
///
/// impl RuntimeClient for ExampleRuntimeClient {
///     fn send_event(&mut self,command: RuntimeCommand) -> RuntimeResponse {
///         RuntimeResponse::Ack
///     }
/// }
///
/// let example_client = ExampleRuntimeClient {};
///
/// let mut submission = Submission::new(manifest,
///                                     1 /* num_parallel_jobs */,
///                                     false /* display_metrics */,
///                                     Arc::new(Mutex::new(example_client)),
///                                     &ExampleDebugClient{},
///                                     true /* enter debugger on start */);
///
/// let mut coordinator = Coordinator::new( 1 /* num_threads */, );
/// coordinator.init();
///
/// coordinator.submit(submission);
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
        }
    }

    /// Initialize the Coordinator
    /// - Setup a control-c signal capture to enter the debugger
    /// - Setup panic hook
    pub fn init(&mut self) {
        #[cfg(feature = "debugger")]
            self.capture_control_c();
    }

    #[cfg(not(target = "wasm32"))]
    #[cfg(feature = "debugger")]
    fn capture_control_c(&self) {
        // Get a reference to the shared control variable that will be moved into the closure
        let requested = self.debug_requested.clone();
        // ignore error as this will be called multiple times by same "program" when running tests and fail
        let _ = ctrlc::set_handler(move || {
            // Set the flag requesting to enter into the debugger to true
            requested.store(true, Ordering::SeqCst);
        });
        debug!("Control-C capture setup to enter debugger");
    }

    /// Start execution of a flow, by submitting a `Submission` to the coordinator
    pub fn submit(&mut self, submission: Submission) {
        self.looper(submission);
    }

    fn looper(&mut self, mut submission: Submission) {
        // This outer loop is just a way of restarting execution from scratch if the debugger requests it
        loop {
            debug!("Resetting stats and initializing all functions");
            submission.state.init();
            submission.runtime_client.lock().unwrap().send_event(Event::FlowStart);

            #[cfg(feature = "debugger")]
            if submission.enter_debugger {
                submission.debugger.enter(&submission.state);
            }

            #[cfg(feature = "metrics")]
                submission.metrics.reset();

            #[cfg(feature = "debugger")]
                let mut display_next_output;
            let mut restart;

            'inner: loop {
                trace!("{}", submission.state);
                #[cfg(feature = "debugger")]
                if self.debug_requested.load(Ordering::SeqCst) {
                    self.debug_requested.store(false, Ordering::SeqCst); // reset to avoid re-entering
                    submission.debugger.enter(&submission.state);
                }

                let debug_check = self.send_jobs(&mut submission);
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

                if submission.state.number_jobs_running() > 0 {
                    match self.job_rx.recv_timeout(submission.job_timeout) {
                        Ok(job) => {
                            #[cfg(feature = "debugger")]
                                {
                                    if display_next_output {
                                        submission.debugger.job_completed(&job);
                                    }
                                }

                            submission.state.complete_job(
                                #[cfg(feature = "metrics")]
                                    &mut submission.metrics,
                                job,
                                #[cfg(feature = "debugger")]
                                    &mut submission.debugger,
                            );
                        }
                        #[cfg(feature = "debugger")]
                        Err(err) => {
                            submission.debugger.panic(&submission.state,
                                                      format!("Error in job reception: '{}'", err));
                        }
                        #[cfg(not(feature = "debugger"))]
                        Err(_) => error!("\tError in Job reception")
                    }
                }

                if submission.state.number_jobs_running() == 0 &&
                    submission.state.number_jobs_ready() == 0 {
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
                            let check = submission.debugger.flow_done(&submission.state);
                            restart = check.1;
                        }
                    }

                if !restart {
                    self.flow_done(&mut submission);
                    return;
                }
            }
        }
    }

    fn flow_done(&self, submission: &mut Submission) {
        debug!("{}", submission.state);

        if submission.display_metrics {
            #[cfg(feature = "metrics")]
            println!("\nMetrics: \n {}", submission.metrics);
            println!("\t\tJobs created: {}\n", submission.state.jobs_created());
        }

        submission.runtime_client.lock().unwrap().send_event(Event::FlowEnd);
    }

    /*
        Send as many jobs as possible for parallel execution.
        Return 'true' if the debugger is requesting a restart
    */
    fn send_jobs(&mut self, submission: &mut Submission) -> (bool, bool) {
        let mut display_output = false;
        let mut restart = false;

        while let Some(job) = submission.state.next_job() {
            match self.send_job(job.clone(), submission) {
                Ok((display, rest)) => {
                    display_output = display;
                    restart = rest;
                }
                Err(err) => {
                    error!("Error sending on 'job_tx': {}", err.to_string());
                    debug!("{}", submission.state);

                    #[cfg(feature = "debuggers")]
                        submission.debugger.error(&submission.state, job);
                }
            }
        }

        (display_output, restart)
    }

    /*
        Send a job for execution
    */
    fn send_job(&self, job: Job, submission: &mut Submission) -> Result<(bool, bool), SendError<Job>> {
        #[cfg(not(feature = "debugger"))]
            let debug_options = (false, false);

        submission.state.start(&job);
        #[cfg(feature = "metrics")]
            submission.metrics.track_max_jobs(submission.state.number_jobs_running());

        #[cfg(feature = "debugger")]
            let debug_options = submission.debugger.check_prior_to_job(&submission.state, job.job_id, job.function_id);

        let job_id = job.job_id;
        self.job_tx.send(job)?;
        debug!("Job #{}:\tSent for execution", job_id);

        Ok(debug_options)
    }
}

#[cfg(test)]
mod test {
    use std::sync::{Arc, Mutex};

    use crate::coordinator::Coordinator;
    use crate::coordinator::Submission;
    #[cfg(feature = "debugger")]
    use crate::debug_client::{DebugClient, Event, Response};
    use crate::manifest::Manifest;
    use crate::manifest::MetaData;
    use crate::runtime_client::Event as RuntimeCommand;
    use crate::runtime_client::Response as RuntimeResponse;
    use crate::runtime_client::RuntimeClient;

    //noinspection DuplicatedCode
    fn test_meta_data() -> MetaData {
        MetaData {
            name: "test".into(),
            version: "0.0.0".into(),
            description: "a test".into(),
            authors: vec!("me".to_string()),
        }
    }

    #[cfg(feature = "debugger")]
    struct TestDebugClient {}

    #[cfg(feature = "debugger")]
    impl DebugClient for TestDebugClient {
        fn send_event(&self, _event: Event) -> Response {
            Response::Ack
        }
    }

    #[cfg(feature = "debugger")]
    fn test_debug_client() -> &'static dyn DebugClient {
        &TestDebugClient {}
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
        let meta_data = test_meta_data();
        let manifest = Manifest::new(meta_data);
        let _ = Submission::new(manifest, 1, true,
                                Arc::new(Mutex::new(TestRuntimeClient {})),
                                #[cfg(feature = "debugger")]
                                    test_debug_client(),
                                #[cfg(feature = "debugger")]
                                    false,
        );
    }

    #[test]
    fn test_flow_done() {
        let meta_data = test_meta_data();
        let manifest = Manifest::new(meta_data);
        let mut submission = Submission::new(manifest, 1, true,
                                             Arc::new(Mutex::new(TestRuntimeClient {})),
                                             #[cfg(feature = "debugger")]
                                                 test_debug_client(),
                                             #[cfg(feature = "debugger")]
                                                 false,
        );
        let mut coordinator = super::Coordinator::new(0);
        coordinator.init();

        coordinator.send_jobs(&mut submission);

        coordinator.flow_done(&mut submission);
    }

    #[test]
    fn test_create() {
        let _ = super::Coordinator::new(0);
    }

    #[test]
    fn test_init() {
        let mut coordinator = super::Coordinator::new(0);
        println!("new worked");
        coordinator.init();
        println!("init worked");
    }

    #[test]
    #[ignore] // Submission currently enters an infinite execution loop so ignore test for now
    fn test_submit() {
        let mut coordinator = Coordinator::new(1);
        coordinator.init();

        let meta_data = test_meta_data();
        let manifest = Manifest::new(meta_data);
        let submission = Submission::new(
            manifest,
            1,
            true,
            Arc::new(Mutex::new(TestRuntimeClient {})),
            #[cfg(feature = "debugger")]
                test_debug_client(),
            #[cfg(feature = "debugger")]
                true,
        );

        coordinator.submit(submission);
    }
}
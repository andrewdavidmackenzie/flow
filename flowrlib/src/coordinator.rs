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

/// A Submission is the struct used to send a flow to the Coordinator for execution. It contains
/// all the information encessary to execute it:
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
pub struct Submission {
    _metadata: MetaData,
    display_metrics: bool,
    #[cfg(feature = "metrics")]
    metrics: Metrics,
    job_timeout: Duration,
    state: RunState,
    #[cfg(feature = "debugger")]
    debugger: Debugger,
    #[cfg(feature = "debugger")]
    enter_debugger: bool,
}

impl Submission {
    /// Create a new `Submission` of a `Flow` for execution with the specified `Manifest`
    /// of `Functions`, executing it with a maximum of `mac_parallel_jobs` running in parallel
    /// connecting via the optional `DebugClient`
    pub fn new(manifest: Manifest,
               max_parallel_jobs: usize,
               display_metrics: bool,
               #[cfg(feature = "debugger")]
               client: &'static dyn DebugClient,
               #[cfg(feature = "debugger")]
               enter_debugger: bool,
    ) -> Submission {
        info!("Maximum jobs dispatched in parallel limited to {}", max_parallel_jobs);
        let output_timeout = Duration::from_secs(1);

        let state = RunState::new(manifest.functions, max_parallel_jobs);

        #[cfg(feature = "metrics")]
            let metrics = Metrics::new(state.num_functions());

        Submission {
            _metadata: manifest.metadata,
            display_metrics,
            #[cfg(feature = "metrics")]
            metrics,
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
    /// A flag that indeicates a request to enter the debugger has been made
    #[cfg(feature = "debugger")]
    debug_requested: Arc<AtomicBool>,
}

/// Create a Submission for a flow to be executed.
/// Instantiate the Coordinator.
/// Send the Submission to the Coordinator to be executed
///
/// # Examples
///
/// ```
/// use std::sync::{Arc, Mutex};
/// use std::io;
/// use std::io::Write;
/// use flowrlib::coordinator::{Coordinator, Submission};
/// use std::process::exit;
/// use flowrlib::manifest::{Manifest, MetaData};
/// #[cfg(any(feature = "debugger"))]
/// use flowrlib::debug_client::{DebugClient, Command, Param, Event, Response, Command::ExitDebugger};
///
/// let meta_data = MetaData {
///                     library_name: "test".into(),
///                     description: "Test submission".into(),
///                     version: "0.0.1".into(),
///                     author_name: "test user".into(),
///                     author_email: "me@acme.com".into()
///                 };
/// let manifest = Manifest::new(meta_data, "fake_dir");
///
/// impl DebugClient for ExampleDebugClient {
///     fn init(&self) {}
///
///     fn get_command(&self, job_number: usize) -> Command { ExitDebugger }
///
///     fn send_event(&self, event: Event) {}
///
///     fn send_response(&self, response: Response) {}
/// }
///
/// let mut submission = Submission::new(manifest,
///                                     1 /* num_parallel_jobs */,
///                                     false /* display_metrics */,
/// #[cfg(any(feature = "debugger"))]
///                                     Debugger::new(ExampleDebugClient),
///                                     true
///                                     );
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

    #[cfg(feature = "debugger")]
    fn capture_control_c(&self) {
        // Get a reference to the shared control variable that will be moved into the closure
        let requested = self.debug_requested.clone();
        ctrlc::set_handler(move || {
            // Set the flag requesting to enter into the debugger to true
            requested.store(true, Ordering::SeqCst);
        }).expect("Error setting Ctrl-C handler");
        debug!("Control-C capture setup to enter debugger");
    }

    /// Start execution of a flow, by submitting a `Submission` to the coordinator
    pub fn submit(&mut self, submission: Submission) {
        self.looper(submission);
    }

    fn looper(&mut self, mut submission: Submission) {
        /*
            This outer loop is just a way of restarting execution from scratch if the debugger
            requests it.
        */
        loop {
            debug!("Resetting stats and initializing all functions");
            submission.state.init();

            #[cfg(feature = "debugger")]
            if submission.enter_debugger {
                submission.debugger.enter(&submission.state);
            }

            #[cfg(feature = "metrics")]
                submission.metrics.reset();

            debug!("===========================    Starting flow execution =============================");
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
                // and restart execution, in the outerloop
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
                    // at the end of exeution, inspect state and possibly reset and rerun
                    break 'inner;
                }
            }

            #[allow(clippy::collapsible_if)]
            if !restart {
                #[cfg(feature = "debugger")]
                    {
                        if submission.enter_debugger {
                            let check = submission.debugger.end(&submission.state);
                            restart = check.1;
                        }
                    }

                if !restart {
                    self.flow_done(&submission);
                    return;
                }
            }
        }
    }

    fn flow_done(&self, submission: &Submission) {
        debug!("=========================== Flow execution ended ======================================");
        debug!("{}", submission.state);

        if submission.display_metrics {
            #[cfg(feature = "metrics")]
            println!("\nMetrics: \n {}", submission.metrics);
            println!("\t\tJobs processed: {}\n", submission.state.jobs());
        }
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
                    submission.state.job_sent();
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
    use crate::coordinator::Coordinator;
    use crate::coordinator::Submission;
    #[cfg(feature = "debugger")]
    use crate::debug_client::{DebugClient, Event, Response};
    #[cfg(feature = "debugger")]
    use crate::debug_client::Command;
    use crate::manifest::Manifest;
    use crate::manifest::MetaData;

    fn test_meta_data() -> MetaData {
        MetaData {
            library_name: "test".into(),
            version: "0.0.0".into(),
            description: "a test".into(),
            author_name: "me".into(),
            author_email: "me@a.com".into(),
        }
    }

    #[cfg(feature = "debugger")]
    struct TestDebugClient {}

    #[cfg(feature = "debugger")]
    impl DebugClient for TestDebugClient {
        fn init(&self) {}

        fn get_command(&self, _job_number: usize) -> Command {
            Command::ExitDebugger
        }

        fn send_event(&self, _event: Event) {}

        fn send_response(&self, _response: Response) {}
    }

    #[cfg(feature = "debugger")]
    fn test_debug_client() -> &'static dyn DebugClient {
        &TestDebugClient {}
    }

    #[test]
    fn create_submission() {
        let meta_data = test_meta_data();
        let manifest = Manifest::new(meta_data, "fake dir");
        let _ = Submission::new(manifest, 1, true,
                                #[cfg(feature = "debugger")]
                                    test_debug_client(),
                                #[cfg(feature = "debugger")]
                                    true,
        );
    }

    #[test]
    fn submit() {
        let mut coordinator = Coordinator::new(1);
        coordinator.init();

        let meta_data = test_meta_data();
        let manifest = Manifest::new(meta_data, "fake dir");
        let submission = Submission::new(manifest, 1, true,
                                         #[cfg(feature = "debugger")]
                                             test_debug_client(),
                                         #[cfg(feature = "debugger")]
                                             true,
        );

        coordinator.submit(submission);
    }

    #[test]
    #[ignore]
    #[cfg(feature = "debugger")]
    fn submit_with_debugger() {
        let mut coordinator = Coordinator::new(1);
        coordinator.init();

        let meta_data = test_meta_data();
        let manifest = Manifest::new(meta_data, "fake dir");
        let submission = Submission::new(manifest, 1, true,
                                         #[cfg(feature = "debugger")]
                                             test_debug_client(),
                                         #[cfg(feature = "debugger")]
                                             true,
        );

        coordinator.submit(submission);
    }
}
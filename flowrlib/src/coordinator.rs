use std::sync::{Arc, Mutex};
use std::sync::mpsc::{Receiver, Sender, SendError};
use std::sync::mpsc;
use std::time::Duration;

use log::Level::Debug;

use crate::debug_client::DebugClient;
use crate::debugger::Debugger;
use crate::execution;
use crate::manifest::Manifest;
use crate::manifest::MetaData;
#[cfg(feature = "metrics")]
use crate::metrics::Metrics;
use crate::run_state::{Job, Output};
use crate::run_state::RunState;

///
/// A Sumission is the struct used to send a flow to the Coordinator for execution. It contains
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
///
pub struct Submission {
    _metadata: MetaData,
    display_metrics: bool,
    #[cfg(feature = "metrics")]
    metrics: Metrics,
    output_timeout: Duration,
    state: RunState,
    #[cfg(feature = "debugger")]
    debugger: Option<Debugger>,
}

impl Submission {
    /*
        Create a new submission
    */
    pub fn new(manifest: Manifest, max_parallel_jobs: usize, display_metrics: bool,
               client: Option<&'static DebugClient>) -> Submission {
        info!("Maximum jobs dispatched in parallel limited to {}", max_parallel_jobs);
        let output_timeout = Duration::from_secs(1);

        let state = RunState::new(manifest.functions, max_parallel_jobs);

        info!("creating metrics");
        #[cfg(feature = "metrics")]
            let metrics = Metrics::new(state.num_functions());

        #[cfg(feature = "debugger")]
            let debugger = match client {
            Some(client) => Some(Debugger::new(client)),
            None => None
        };

        Submission {
            _metadata: manifest.metadata,
            display_metrics,
            #[cfg(feature = "metrics")]
            metrics,
            output_timeout,
            state,
            #[cfg(feature = "debugger")]
            debugger,
        }
    }
}

///
/// The Coordinator is responsible for coordinating the dispatching of jobs (consisting
/// of a set of Input values and an Implementation of a Function) for execution,
/// gathering the resulting Outputs and distributing output values to other connected function's
/// Inputs.
///
/// It accepts Flows to be executed in the form of a Submission struct that has the required
/// information to execut the flow.
///
pub struct Coordinator {
    num_threads: usize,
    job_tx: Sender<Job>,
    output_rx: Receiver<Output>,
    pure_job_tx: Sender<Job>,
    pure_job_rx: Receiver<Job>,
    output_tx: Sender<Output>,
}

/// Create a Submission for a flow to be executed.
/// Instantiate the Coordinator.
/// Send the Submission to the Coordinator to be executed
///
/// # Example
/// ```
/// use std::sync::{Arc, Mutex};
/// use std::io;
/// use std::io::Write;
/// use flowrlib::coordinator::{Coordinator, Submission};
/// use std::process::exit;
/// use flowrlib::manifest::{Manifest, MetaData};
/// use flowrlib::debug_client::{Command, Param};
/// use flowrlib::debug_client::Event;
/// use flowrlib::debug_client::Response;
///
/// let meta_data = MetaData {
///                     alias: "test flow".into(),
///                     version: "0.0.1".into(),
///                     author_name: "test user".into(),
///                     author_email: "me@acme.com".into()
///                 };
/// let manifest = Manifest::new(meta_data);
///
/// let mut submission = Submission::new(manifest,
///                                     1 /* num_parallel_jobs */,
///                                     false /* display_metrics */,
///                                     None /* debug client*/);
///
/// let mut coordinator = Coordinator::new( 1 /* num_threads */, );
///
/// coordinator.submit(submission);
///
/// exit(0);
/// ```
///
impl Coordinator {
    pub fn new(num_threads: usize) -> Self {
        let (job_tx, job_rx, ) = mpsc::channel();
        let (pure_job_tx, pure_job_rx, ) = mpsc::channel();
        let (output_tx, output_rx) = mpsc::channel();

        if num_threads > 0 {
            info!("Starting {} additional executor threads", num_threads);
            let shared_job_receiver = Arc::new(Mutex::new(job_rx));
            execution::start_executors(num_threads, &shared_job_receiver, &output_tx);
        }

        let coordinator = Coordinator {
            num_threads,
            job_tx,
            output_rx,
            pure_job_tx,
            pure_job_rx,
            output_tx,
        };

        coordinator
    }

    /*
        Start execution of a flow, by sending the submission to the looper thread
    */
    pub fn submit(&mut self, submission: Submission) {
        self.looper(submission);
    }

    fn looper(&mut self, mut submission: Submission) {
        execution::set_panic_hook();

        /*
            This outer loop is just a way of restarting execution from scratch if the debugger
            requests it.
        */
        loop {
            debug!("Resetting stats and initializing all functions");
            submission.state.init();

            if cfg!(feature = "debugger") {
                if let Some(ref mut debugger) = submission.debugger {
                    debugger.start(&submission.state);
                }
            }

            if cfg!(feature = "metrics") {
                submission.metrics.reset();
            }

            debug!("Starting flow execution");
            let mut display_next_output;
            let mut restart;

            'inner: loop {
                let debug_check = self.send_jobs(&mut submission);
                display_next_output = debug_check.0;
                restart = debug_check.1;

                if restart {
                    break 'inner;
                }

                // see if any jobs on the pure_job channel that this main thread should execute
                let _ = execution::get_and_execute_pure_job(&self.pure_job_rx, &self.output_tx);

                if submission.state.number_jobs_running() > 0 {
                    match self.output_rx.recv_timeout(submission.output_timeout) {
                        Ok(output) => {
                            submission.state.job_done(&output);

                            debug!("\tCompleted Job #{} for Function #{}", output.job_id, output.function_id);
                            if cfg!(feature = "debugger") && display_next_output {
                                if let Some(ref mut debugger) = submission.debugger {
                                    debugger.job_completed(&output);
                                }
                            }

                            submission.state.process_output(&mut submission.metrics, output, &mut submission.debugger)
                        }
                        Err(err) => error!("Error receiving execution result: {}", err)
                    }
                }

                if submission.state.number_jobs_running() == 0 &&
                    submission.state.number_jobs_ready() == 0 {
                    // execution is done - but not returning here allows us to go into debugger
                    // at the end of exeution, inspect state and possibly reset and rerun
                    break 'inner;
                }
            }

            if !restart {
                if cfg!(feature = "debugger") {
                    if let Some(ref mut debugger) = submission.debugger {
                        let check = debugger.end(&submission.state);
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

    fn flow_done(&self, submission: &Submission) {
        debug!("Flow execution ended, no remaining function ready to run");

        if cfg!(feature = "logging") && log_enabled!(Debug) {
            debug!("{}", submission.state);
        }

        if submission.display_metrics {
            #[cfg(feature = "metrics")]
            println!("\nMetrics: \n {}", submission.metrics);
            println!("\t\tJobs processed: \t{}\n", submission.state.jobs());
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
            match self.send_job(job, submission) {
                Ok((display, rest)) => {
                    debug!("Job sent to Executors");
                    submission.state.job_sent();
                    display_output = display;
                    restart = rest;
                }
                Err(err) => {
                    error!("Error sending on 'job_tx': {}", err.to_string());

                    if cfg!(feature = "logging") && log_enabled!(Debug) {
                        debug!("{}", submission.state);
                    }

                    if let Some(ref mut debugger) = submission.debugger {
                        debugger.error(&submission.state, err.to_string());
                    }
                }
            }
        }

        (display_output, restart)
    }

    /*
        Send a job for execution:
        - if impure, then needs to be run on a thread which has stdio so send on the 'pure_job' channel
        - if pure send it on the 'job' channel where executors will pick it up by an executor

        - In the case there are no executor threads, then we need to have all jobs executed on the same
          thread, so send it on the 'pure_job' channel always.
    */
    fn send_job(&self, job: Job, submission: &mut Submission) -> (Result<(bool, bool), SendError<Job>>) {
        let mut debug_options = (false, false);

        submission.state.start(&job);
        #[cfg(feature = "metrics")]
            submission.metrics.track_max_jobs(submission.state.number_jobs_running());

        if cfg!(feature = "debugger") {
            if let Some(ref mut debugger) = submission.debugger {
                debug_options = debugger.check_prior_to_job(&submission.state, job.job_id, job.function_id);
            }
        }

        // If there are no executor threads then send all jobs on pure_job channel to have main thread execute them
        if self.num_threads == 0 || job.impure {
            self.pure_job_tx.send(job)?;
        } else {
            self.job_tx.send(job)?;
        }

        Ok(debug_options)
    }
}

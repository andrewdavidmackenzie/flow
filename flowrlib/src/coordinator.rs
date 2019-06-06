use std::sync::mpsc::{Receiver, Sender};
use std::sync::mpsc;
use std::time::Duration;

use debug_client::DebugClient;
use debugger::Debugger;
use execution;
use log::Level::Debug;
use manifest::Manifest;
#[cfg(feature = "metrics")]
use metrics::Metrics;
use run_state::{Job, Output};
use run_state::RunState;
use manifest::MetaData;

/*
    RunList is a structure that maintains the state of all the functions in the currently
    executing flow.

    A function maybe blocking multiple others trying to send data to it.
    Those others maybe blocked trying to send to multiple different function.

    function:
    A list of all the functions that could be executed at some point.

    inputs_satisfied:
    A list of functions who's inputs are satisfied.

    blocking:
    A list of tuples of function ids where first id is id of the function data is trying to be sent
    to, and the second id is the id of the function trying to send data.

    ready:
    A list of Processs who are ready to be run, they have their inputs satisfied and they are not
    blocked on the output (so their output can be produced).
*/
pub struct Coordinator {
    job_tx: Sender<Job>,
    output_rx: Receiver<Output>,
}

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
    pub fn new(manifest: Manifest, max_parallel_jobs: usize, display_metrics: bool,
               client: Option<&'static DebugClient>) -> Submission {
        info!("Max Jobs in parallel set to {}", max_parallel_jobs);
        let output_timeout = Duration::new(1, 0);

        let state = RunState::new(manifest.functions, max_parallel_jobs);
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

/// The generated code for a flow consists of a list of Functions.
///
/// This list is built program start-up in `main` which then starts execution of the flow by calling
/// this `execute` method.
///
/// You should not have to write code to call `execute` yourself, it will be called from the
/// generated code in the `main` method.
///
/// On completion of the execution of the flow it will return and `main` will call `exit`
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
/// let mut coordinator = Coordinator::new( 1 /* num_threads */, );
///
/// let mut submission = Submission::new(manifest,
///                                     1 /* num_parallel_jobs */,
///                                     false /* display_metrics */,
///                                     None /* debug client*/);
///
/// coordinator.submit(submission);
///
/// exit(0);
/// ```
impl Coordinator {
    pub fn new(num_threads: usize) -> Self {
        let (job_tx, job_rx, ) = mpsc::channel();
        let (output_tx, output_rx) = mpsc::channel();

        info!("Starting {} executor threads", num_threads);
        execution::start_executors(num_threads, job_rx, output_tx.clone());

        let coordinator = Coordinator {
            job_tx,
            output_rx,
        };

        // Start a thread that executes the looper that waits for and executes flows

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
            let check = self.send_job(job, submission);
            display_output = check.0;
            restart = check.1;
        }

        (display_output, restart)
    }

    /*
        Send a job for execution:
        - if impure, then needs to be run on the main thread which has stdio (stdin in particular)
        - if pure send it on the 'job_tx' channel where executors will pick it up by an executor
    */
    fn send_job(&self, job: Job, submission: &mut Submission) -> (bool, bool) {
        let mut debug_options = (false, false);

        submission.state.start(&job);
        #[cfg(feature = "metrics")]
            submission.metrics.track_max_jobs(submission.state.number_jobs_running());

        if cfg!(feature = "debugger") {
            if let Some(ref mut debugger) = submission.debugger {
                debug_options = debugger.check_job(&submission.state, job.job_id, job.function_id);
            }
        }

        match self.job_tx.send(job) {
            Ok(_) => debug!("Job sent to Executors"),
            Err(err) => error!("Error sending on 'job_tx': {}", err)
        }

        if cfg!(feature = "logging") && log_enabled!(Debug) {
            debug!("{}", submission.state);
        }

        debug_options
    }
}

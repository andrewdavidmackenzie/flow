use log::Level::Debug;
#[cfg(feature = "debugger")]
use debugger::Debugger;
#[cfg(feature = "metrics")]
use metrics::Metrics;
use debug_client::DebugClient;
use run_state::RunState;
use execution;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::mpsc;
use std::time::Duration;
use run_state::{Job, Output};
use manifest::Manifest;

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
    debugging: bool,
    display_metrics: bool,

    #[cfg(feature = "debugger")]
    pub debugger: Debugger,

    job_tx: Sender<Job>,
    output_rx: Receiver<Output>,
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
/// use flowrlib::coordinator::Coordinator;
/// use std::process::exit;
/// use flowrlib::debug_client::DebugClient;
/// use flowrlib::manifest::{Manifest, MetaData};
///
/// struct SampleDebugClient {}
///
/// impl DebugClient for SampleDebugClient {
///    fn init(&self) {}
///
///    fn display(&self, output: &str) {
///        print!("{}", output);
///        io::stdout().flush().unwrap();
///    }
///
///    fn read_input(&self, input: &mut String) -> io::Result<usize> {
///        io::stdin().read_line(input)
///    }
/// }
///
/// let meta_data = MetaData {
///                     alias: "test flow".into(),
///                     version: "0.0.1".into(),
///                     author_name: "test user".into(),
///                     author_email: "me@acme.com".into()
///                 };
/// let manifest = Manifest::new(meta_data);
///
/// let mut coordinator = Coordinator::new(&SampleDebugClient{},
///                                    1 /* num_threads */,
///                                    false /* use_debugger */,
///                                    false /* display_metrics */);
///
/// coordinator.run(manifest, 1 /* num_parallel_jobs */);
///
/// exit(0);
/// ```
impl Coordinator {
    pub fn new(client: &'static DebugClient, num_threads: usize, debugging: bool, display_metrics: bool) -> Self {
        let (job_tx, job_rx, ) = mpsc::channel();
        let (output_tx, output_rx) = mpsc::channel();

        debug!("Starting {} executor threads", num_threads);
        execution::start_executors(num_threads, job_rx, output_tx.clone());

        execution::set_panic_hook();

        Coordinator {
            debugging,
            display_metrics,
            #[cfg(feature = "debugger")]
            debugger: Debugger::new(client),
            job_tx,
            output_rx,
        }
    }

    pub fn run(&mut self, manifest: Manifest, num_parallel_jobs: usize) {
        let output_timeout = Duration::new(1, 0);
        let mut state = RunState::new(manifest.functions, num_parallel_jobs);
        #[cfg(feature = "metrics")]
            let mut metrics = Metrics::new(state.num_functions());

        /*
            This outer loop is just a way of restarting execution from scratch if the debugger
            requests it.
        */
        loop {
            debug!("Initializing all functions");
            state.init();

            if cfg!(feature = "metrics") {
                metrics.reset();
            }

            debug!("Starting flow execution");
            let mut display_next_output;
            let mut restart;

            if cfg!(feature = "debugger") && self.debugging {
                self.debugger.start(&mut state);
            }

            'inner: loop {
                let debug_check = self.send_jobs(&mut state, &mut metrics);
                display_next_output = debug_check.0;
                restart = debug_check.1;

                if restart {
                    break 'inner;
                }

                if state.number_jobs_running() > 0 {
                    match self.output_rx.recv_timeout(output_timeout) {
                        Ok(output) => {
                            state.done(&output);
                            state.process_output(&mut metrics, output,
                                                 display_next_output,
                                                 &mut self.debugger)
                        },
                        Err(err) => error!("Error receiving execution result: {}", err)
                    }
                }

                if state.number_jobs_running() == 0 &&
                    state.number_jobs_ready() == 0 {
                    // execution is done - but not returning here allows us to go into debugger
                    // at the end of exeution, inspect state and possibly reset and rerun
                    break 'inner;
                }
            }

            if !restart {
                if cfg!(feature = "logging") && log_enabled!(Debug) {
                    debug!("{}", state);
                }

                if cfg!(feature = "debugger") && self.debugging {
                    let check = self.debugger.end(&mut state);
                    restart = check.1;
                }

                if !restart {
                    self.flow_done(&metrics, &state);
                    return;
                }
            }
        }
    }

    fn flow_done(&self, metrics: &Metrics, state: &RunState) {
        debug!("Flow execution ended, no remaining function ready to run");
        if self.display_metrics {
            #[cfg(feature = "metrics")]
            println!("\nMetrics: \n {}", metrics);
            println!("\t\tJobs processed: \t{}\n", state.jobs());
        }
    }

    /*
        Send as many jobs as possible for parallel execution.
        Return 'true' if the debugger is requesting a restart
    */
    fn send_jobs(&mut self, state: &mut RunState, metrics: &mut Metrics) -> (bool, bool) {
        let mut display_output = false;
        let mut restart = false;

        while let Some(job) = state.next_job() {
            display_output = false;
            restart = false;

            if cfg!(feature = "logging") && log_enabled!(Debug) {
                debug!("{}", state);
            }

            if cfg!(feature = "debugger") && self.debugging {
                let check = self.debugger.check(state, job.job_id, job.function_id);
                display_output = check.0;
                restart = check.1;

                if restart {
                    return (display_output, restart);
                }
            }

            self.send_job(state, metrics, job);
        }

        (display_output, restart)
    }

    /*
        Send a job for execution:
        - if impure, then needs to be run on the main thread which has stdio (stdin in particular)
        - if pure send it on the 'job_tx' channel where executors will pick it up by an executor
    */
    fn send_job(&self, state: &mut RunState, metrics: &mut Metrics, job: Job) {
        state.start(&job);
        #[cfg(feature = "metrics")]
        metrics.track_max_jobs(state.number_jobs_running());

        match self.job_tx.send(job) {
            Ok(_) => debug!("Job sent to Executors"),
            Err(err) => error!("Error sending on 'job_tx': {}", err)
        }
    }
}

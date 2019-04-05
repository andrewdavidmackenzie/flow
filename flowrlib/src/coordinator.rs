use function::Function;
use serde_json::Value;
use std::sync::{Arc, Mutex};
use log::Level::Debug;
#[cfg(feature = "debugger")]
use debugger::Debugger;
#[cfg(feature = "metrics")]
use std::fmt;
#[cfg(feature = "metrics")]
use std::time::Instant;
use debug_client::DebugClient;
use run_state::RunState;
use implementation::Implementation;
use execution;
use std::sync::mpsc::{SyncSender, Receiver, Sender};
use std::sync::mpsc;
use std::time::Duration;
use std::cmp::max;

#[cfg(feature = "metrics")]
struct Metrics {
    num_functions: usize,
    outputs_sent: u32,
    start_time: Instant,
    max_simultaneous_jobs: usize
}

#[cfg(feature = "metrics")]
impl Metrics {
    fn new() -> Self {
        Metrics {
            num_functions: 0,
            outputs_sent: 0,
            start_time: Instant::now(),
            max_simultaneous_jobs: 0
        }
    }

    fn reset(&mut self, num_functions: usize) {
        self.num_functions = num_functions;
        self.outputs_sent = 0;
        self.start_time = Instant::now();
        self.max_simultaneous_jobs = 0;
    }
}

#[cfg(feature = "metrics")]
impl fmt::Display for Metrics {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let elapsed = self.start_time.elapsed();
        write!(f, "\t Number of Functions: \t{}\n", self.num_functions)?;
        write!(f, "\t        Outputs sent: \t{}\n", self.outputs_sent)?;
        write!(f, "\t     Elapsed time(s): \t{:.*}\n", 9, elapsed.as_secs() as f64 + elapsed.subsec_nanos() as f64 * 1e-9)?;
        write!(f, "\tMax Jobs in Parallel: \t{}", self.max_simultaneous_jobs)
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
/// use flowrlib::function::Function;
/// use flowrlib::coordinator::run;
/// use std::process::exit;
/// use flowrlib::debug_client::DebugClient;
///
/// struct CLIDebugClient {}
///
/// impl DebugClient for CLIDebugClient {
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
/// const CLI_DEBUG_CLIENT: &DebugClient = &CLIDebugClient{};
///
/// let mut functions = Vec::<Arc<Mutex<Function>>>::new();
///
/// run(functions, false /* print_metrics */, CLI_DEBUG_CLIENT,
///     false /* use_debugger */, 1 /* threads */);
///
/// exit(0);
/// ```
pub fn run(functions: Vec<Arc<Mutex<Function>>>, display_metrics: bool,
           client: &'static DebugClient, use_debugger: bool, num_threads: usize) {
    let mut run_list = Coordinator::new(client, functions, use_debugger, num_threads);

    run_list.run();

    if display_metrics {
        #[cfg(feature = "metrics")]
            run_list.print_metrics();
        println!("\t\tJobs sent: \t{}\n", run_list.state.jobs());
    }
}

pub struct Job {
    pub function_id: usize,
    pub implementation: Arc<Implementation>,
    pub input_values: Vec<Vec<Value>>,
    pub destinations: Vec<(String, usize, usize)>,
    pub impure: bool,
}

#[derive(Debug)]
pub struct Output {
    pub function_id: usize,
    pub input_values: Vec<Vec<Value>>,
    pub result: (Option<Value>, bool),
    pub destinations: Vec<(String, usize, usize)>,
    pub error: Option<String>,
}

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
struct Coordinator {
    pub state: RunState,

    #[cfg(feature = "metrics")]
    metrics: Metrics,

    debugging: bool,
    #[cfg(feature = "debugger")]
    pub debugger: Debugger,

    job_tx: SyncSender<Job>,
    output_rx: Receiver<Output>,
    output_tx: Sender<Output>,
}

impl Coordinator {
    fn new(client: &'static DebugClient, functions: Vec<Arc<Mutex<Function>>>,
           debugging: bool, num_threads: usize) -> Self {
        let (job_tx, job_rx, ) = mpsc::sync_channel(2 * num_threads);
        let (output_tx, output_rx) = mpsc::channel();

        debug!("Starting Coordinator and {} executor threads", num_threads);
        execution::start_executors(num_threads, job_rx, output_tx.clone());

        Coordinator {
            state: RunState::new(functions),
            #[cfg(feature = "metrics")]
            metrics: Metrics::new(),
            debugging,
            #[cfg(feature = "debugger")]
            debugger: Debugger::new(client),
            job_tx,
            output_rx,
            output_tx,
        }
    }

    fn run(&mut self) {
        let output_timeout = Duration::new(1, 0);
        let mut display_output;
        let mut restart;

        execution::set_panic_hook();

        'outer: loop {
            debug!("Initializing all functions");
            let num_functions = self.state.init();

            if cfg!(feature = "metrics") {
                self.metrics.reset(num_functions);
            }

            debug!("Starting flow execution");
            display_output = false;
            restart = false;

            'inner: while let Some(id) = self.state.next() {
                #[cfg(feature = "metrics")]
                self.track_max_jobs();

                if log_enabled!(Debug) {
                    self.state.print();
                }

                if cfg!(feature = "debugger") && self.debugging {
                    let check = self.debugger.check(&mut self.state, id);
                    display_output = check.0;
                    restart = check.1;

                    if restart {
                        break 'inner;
                    }
                }

                let job = self.create_job(id);

                self.send_job(job);

                // TODO move the reception onto another thread?
                match self.output_rx.recv_timeout(output_timeout) {
                    Ok(output) => self.update_states(output, display_output),
                    Err(err) => error!("Error receiving execution result: {}", err)
                }
            }

            if !restart {
                if cfg!(feature = "logging") && log_enabled!(Debug) {
                    self.state.print();
                }

                if cfg!(feature = "debugger") && self.debugging {
                    let check = self.debugger.end(&mut self.state);
                    restart = check.1;
                }

                if !restart {
                    break 'outer; // We're done!
                }
            }
        }

        debug!("Flow execution ended, no remaining function ready to run");
    }

    /*
        Send a job for execution:
        - if impure, then needs to be run on the main thread which has stdio (stdin in particular)
        - if pure send it on the 'job_tx' channel where executors will pick it up by an executor
    */
    fn send_job(&self, job: Job) {
        if job.impure {
            debug!("Job executed on main thread");
            execution::execute(job, &self.output_tx);
        } else {
            match self.job_tx.send(job) {
                Ok(_) => debug!("Job sent to Executors"),
                Err(err) => error!("Error sending on 'job_tx': {}", err)
            }
        };
    }

    /*
        Given a function id, prepare a job for execution that contains the input values, the
        implementation and the destination functions the output should be sent to when done
    */
    fn create_job(&mut self, id: usize) -> Job {
        let function_arc = self.state.get(id);
        let function: &mut Function = &mut *function_arc.lock().unwrap();

        let input_values = function.get_input_values();

        self.state.unblock_senders_to(id);
        debug!("Preparing Job for Function #{} '{}' with inputs: {:?}", id, function.name(), input_values);

        let implementation = function.get_implementation();

        #[cfg(any(feature = "metrics", feature = "debugger"))]
            self.state.increment_jobs();

        let destinations = function.output_destinations().clone();

        Job { function_id: id, implementation, input_values, destinations, impure: function.is_impure() }
    }

    /*
        Take an output produced by a function and modify the runlist accordingly
        If other functions were blocked trying to send to this one - we can now unblock them
        as it has consumed it's inputs and they are free to be sent to again.

        Then take the output and send it to all destination IOs on different function it should be
        sent to, marking the source function as blocked because those others must consume the output
        if those other function have all their inputs, then mark them accordingly.
    */
    fn update_states(&mut self, output: Output, display_output: bool) {
        match output.error {
            None => {
                let output_value = output.result.0;
                let source_can_run_again = output.result.1;

                debug!("\tCompleted Function #{}", output.function_id);
                if cfg!(feature = "debugger") & &display_output {
                    self.debugger.client.display(&format!("Completed Function #{}\n", output.function_id));
                }

                // did it produce any output value
                if let Some(output_v) = output_value {
                    debug!("\tProcessing output '{}' from Function #{}", output_v, output.function_id);

                    if cfg!(feature = "debugger") & &display_output {
                        self.debugger.client.display(&format!("\tProduced output {}\n", &output_v));
                    }

                    for (ref output_route, destination_id, io_number) in output.destinations {
                        let destination_arc = self.state.get(destination_id);
                        let mut destination = destination_arc.lock().unwrap();
                        let output_value = output_v.pointer(&output_route).unwrap();
                        debug!("\t\tFunction #{} sent value '{}' via output route '{}' to Function #{} '{}' input :{}",
                               output.function_id, output_value, output_route, &destination_id, destination.name(), &io_number);
                        if cfg!(feature = "debugger") & &display_output {
                            self.debugger.client.display(
                                &format!("\t\tSending to {}:{}\n", destination_id, io_number));
                        }

                        #[cfg(feature = "debugger")]
                            self.debugger.watch_data(&mut self.state, output.function_id, output_route,
                                                     &output_value, destination_id, io_number);

                        destination.write_input(io_number, output_value.clone());

                        #[cfg(feature = "metrics")]
                            self.increment_outputs_sent();

                        if destination.input_full(io_number) {
                            self.state.set_blocked_by(destination_id, output.function_id);
                            #[cfg(feature = "debugger")]
                                self.debugger.check_block(&mut self.state, destination_id, output.function_id);
                        }

                        // for the case when a function is sending to itself, delay determining if it should
                        // be in the blocked or will_run lists until it has sent all it's other outputs
                        // as it might be blocked by another function.
                        // Iif not, this will be fixed in the "if source_can_run_again {" block below
                        if destination.can_run() & &(output.function_id != destination_id) {
                            self.state.inputs_ready(destination_id);
                        }
                    }
                }

                // if it wants to run again, and after possibly refreshing any constant inputs, it can
                // (it's inputs are ready) then add back to the Will Run list
                if source_can_run_again {
                    let source_arc = self.state.get(output.function_id);
                    let mut source = source_arc.lock().unwrap();

                    // refresh any constant inputs it may have
                    source.refresh_constant_inputs();

                    if source.can_run() {
                        self.state.inputs_ready(output.function_id);
                    }
                }
            }
            Some(_) => error!("Error in Job execution:\n{:?}", output)
        }

        // remove from the running list
        self.state.done(output.function_id);
    }

    #[cfg(feature = "metrics")]
    fn print_metrics(&self) {
        println!("\nMetrics: \n {}", self.metrics);
    }

    #[cfg(feature = "metrics")]
    fn increment_outputs_sent(&mut self) {
        self.metrics.outputs_sent += 1;
    }

    #[cfg(feature = "metrics")]
    fn track_max_jobs(&mut self) {
        let jobs_running = self.state.number_jobs_running();
        self.metrics.max_simultaneous_jobs = max(self.metrics.max_simultaneous_jobs, jobs_running);
    }
}

#[test]
fn test_metrics_reset() {
    let mut metrics = Metrics::new();
    metrics.outputs_sent = 10;
    metrics.reset(10);
    assert_eq!(metrics.outputs_sent, 0);
    assert_eq!(metrics.num_functions, 10);
}


use function::Function;
use serde_json::Value;
use std::sync::Arc;
use log::Level::Debug;
#[cfg(feature = "debugger")]
use debugger::Debugger;
#[cfg(feature = "metrics")]
use metrics::Metrics;
use debug_client::DebugClient;
use flow::Flow;
use run_state::RunState;
use implementation::Implementation;
use execution;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::mpsc;
use std::time::Duration;

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
/// use flowrlib::flow::Flow;
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
/// let mut flow = Flow::new(&manifest);
///
/// let mut coordinator = Coordinator::new(&SampleDebugClient{},
///                                    1 /* num_threads */,
///                                    false /* use_debugger */,
///                                    false /* display_metrics */);
///
/// coordinator.run(flow, 1 /* num_parallel_jobs */);
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

    pub fn run(&mut self, flow: Flow, num_parallel_jobs: usize) {
        let output_timeout = Duration::new(1, 0);
        let mut state = RunState::new(flow.functions, num_parallel_jobs);
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

            'inner: loop {
                let debug_check = self.send_jobs(&mut state, &mut metrics);
                display_next_output = debug_check.0;
                restart = debug_check.1;

                if restart {
                    break 'inner;
                }

                if state.number_jobs_running() > 0 {
                    match self.output_rx.recv_timeout(output_timeout) {
                        Ok(output) => self.update_states(&mut state, &mut metrics,
                                                         output, display_next_output),
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
            println!("\t\t   Jobs sent: \t{}\n", state.jobs());
        }
    }

    /*
        Send as many jobs as possible for parallel execution.
        Return 'true' if the debugger is requesting a restart
    */
    fn send_jobs(&mut self, state: &mut RunState, metrics: &mut Metrics) -> (bool, bool) {
        let mut display_output = false;
        let mut restart = false;

        while let Some(id) = state.next() {
            display_output = false;
            restart = false;

            #[cfg(feature = "metrics")]
                metrics.track_max_jobs(state.number_jobs_running());

            if cfg!(feature = "logging") && log_enabled!(Debug) {
                debug!("{}", state);
            }

            if cfg!(feature = "debugger") && self.debugging {
                let check = self.debugger.check(state, id);
                display_output = check.0;
                restart = check.1;

                if restart {
                    return (display_output, restart);
                }
            }

            let job = self.create_job(state, id);
            self.send_job(job);
        }

        (display_output, restart)
    }

    /*
        Send a job for execution:
        - if impure, then needs to be run on the main thread which has stdio (stdin in particular)
        - if pure send it on the 'job_tx' channel where executors will pick it up by an executor
    */
    fn send_job(&self, job: Job) {
        match self.job_tx.send(job) {
            Ok(_) => debug!("Job sent to Executors"),
            Err(err) => error!("Error sending on 'job_tx': {}", err)
        }
    }

    /*
        Given a function id, prepare a job for execution that contains the input values, the
        implementation and the destination functions the output should be sent to when done
    */
    fn create_job(&mut self, state: &mut RunState, id: usize) -> Job {
        let function_arc = state.get(id);
        let function: &mut Function = &mut *function_arc.lock().unwrap();

        let input_values = function.take_input_values();

        state.unblock_senders_to(id);
        debug!("Preparing Job for Function #{} '{}' with inputs: {:?}", id, function.name(), input_values);

        let implementation = function.get_implementation();

        #[cfg(any(feature = "metrics", feature = "debugger"))]
            state.increment_jobs();

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
    fn update_states(&mut self, state: &mut RunState, metrics: &mut Metrics,
                     output: Output, display_output: bool) {
        match output.error {
            None => {
                let output_value = output.result.0;
                let source_can_run_again = output.result.1;

                debug!("\tCompleted Function #{}", output.function_id);
                if cfg!(feature = "debugger") && display_output {
                    self.debugger.client.display(&format!("Completed Function #{}\n", output.function_id));
                }

                // did it produce any output value
                if let Some(output_v) = output_value {
                    debug!("\tProcessing output '{}' from Function #{}", output_v, output.function_id);

                    if cfg!(feature = "debugger") && display_output {
                        self.debugger.client.display(&format!("\tProduced output {}\n", &output_v));
                    }

                    for (ref output_route, destination_id, io_number) in output.destinations {
                        let destination_arc = state.get(destination_id);
                        let mut destination = destination_arc.lock().unwrap();
                        let output_value = output_v.pointer(&output_route).unwrap();
                        debug!("\t\tFunction #{} sent value '{}' via output route '{}' to Function #{} '{}' input :{}",
                               output.function_id, output_value, output_route, &destination_id, destination.name(), &io_number);
                        if cfg!(feature = "debugger") && display_output {
                            self.debugger.client.display(
                                &format!("\t\tSending to {}:{}\n", destination_id, io_number));
                        }

                        #[cfg(feature = "debugger")]
                            self.debugger.watch_data(state, output.function_id, output_route,
                                                     &output_value, destination_id, io_number);

                        destination.write_input(io_number, output_value.clone());

                        #[cfg(feature = "metrics")]
                            metrics.increment_outputs_sent();

                        if destination.input_full(io_number) {
                            state.set_blocked_by(destination_id, output.function_id);
                            #[cfg(feature = "debugger")]
                                self.debugger.check_block(state, destination_id, output.function_id);
                        }

                        // for the case when a function is sending to itself, delay determining if it should
                        // be in the blocked or ready lists until it has sent all it's other outputs
                        // as it might be blocked by another function.
                        // Iif not, this will be fixed in the "if source_can_run_again {" block below
                        if destination.inputs_full() && (output.function_id != destination_id) {
                            state.inputs_now_full(destination_id);
                        }
                    }
                }

                // if it wants to run again, and after possibly refreshing any constant inputs, it can
                // (it's inputs are ready) then add back to the Will Run list
                if source_can_run_again {
                    let source_arc = state.get(output.function_id);
                    let mut source = source_arc.lock().unwrap();

                    // refresh any constant inputs it may have
                    source.init_inputs(false);

                    if source.inputs_full() {
                        state.inputs_now_full(output.function_id);
                    }
                }
            }
            Some(_) => error!("Error in Job execution:\n{:?}", output)
        }

        // remove from the running list
        state.done(output.function_id);
    }
}

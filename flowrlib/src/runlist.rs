use function::Function;
use serde_json::Value;
use std::panic::RefUnwindSafe;
use std::panic::UnwindSafe;
use std::sync::{Arc, Mutex};
use log::LogLevel::Debug;
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
use std::sync::mpsc::{SyncSender, Receiver};
use std::sync::mpsc;

#[cfg(feature = "metrics")]
struct Metrics {
    num_processs: usize,
    outputs_sent: u32,
    start_time: Instant,
}

#[cfg(feature = "metrics")]
impl Metrics {
    fn new() -> Self {
        Metrics {
            num_processs: 0,
            outputs_sent: 0,
            start_time: Instant::now(),
        }
    }

    fn reset(&mut self, num_processes: usize) {
        self.num_processs = num_processes;
        self.outputs_sent = 0;
        self.start_time = Instant::now();
    }
}

#[cfg(feature = "metrics")]
impl fmt::Display for Metrics {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let elapsed = self.start_time.elapsed();
        write!(f, "\t\tNumber of Functions: \t{}\n", self.num_processs)?;
        write!(f, "\t\tOutputs sent: \t\t{}\n", self.outputs_sent)?;
        write!(f, "\t\tElapsed time(s): \t{:.*}", 9, elapsed.as_secs() as f64 + elapsed.subsec_nanos() as f64 * 1e-9)
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
/// use flowrlib::runlist::run;
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
/// let mut processes = Vec::<Arc<Mutex<Function>>>::new();
///
/// run(processes, false /* print_metrics */, CLI_DEBUG_CLIENT, false /* use_debugger */);
///
/// exit(0);
/// ```
pub fn run(processes: Vec<Arc<Mutex<Function>>>, display_metrics: bool,
           client: &'static DebugClient, use_debugger: bool) {
    let mut run_list = RunList::new(client, processes, use_debugger);

    run_list.run();

    if display_metrics {
        #[cfg(feature = "metrics")]
            run_list.print_metrics();
        println!("\t\tFunction dispatches: \t{}\n", run_list.state.dispatches());
    }
}

pub struct Dispatch {
    pub id: usize,
    pub implementation: Arc<Implementation>,
    pub input_values: Vec<Vec<Value>>,
    pub destinations: Vec<(String, usize, usize)>,
    pub impure: bool,
}

pub struct Output {
    pub id: usize,
    pub input_values: Vec<Vec<Value>>,
    pub result: (Option<Value>, bool),
    pub destinations: Vec<(String, usize, usize)>,
}

/*
    RunList is a structure that maintains the state of all the processs in the currently
    executing flow.

    A process maybe blocking multiple others trying to send data to it.
    Those others maybe blocked trying to send to multiple different processs.

    processs:
    A list of all the processs that could be executed at some point.

    inputs_satisfied:
    A list of processs who's inputs are satisfied.

    blocking:
    A list of tuples of process ids where first id is id of the process data is trying to be sent
    to, and the second id is the id of the process trying to send data.

    ready:
    A list of Processs who are ready to be run, they have their inputs satisfied and they are not
    blocked on the output (so their output can be produced).
*/
struct RunList {
    pub state: RunState,

    #[cfg(feature = "metrics")]
    metrics: Metrics,

    debugging: bool,
    #[cfg(feature = "debugger")]
    pub debugger: Debugger,

    dispatch_tx: SyncSender<Dispatch>,
    output_rx: Receiver<Output>,
}

impl RefUnwindSafe for RunList {}

impl UnwindSafe for RunList {}

impl RunList {
    fn new(client: &'static DebugClient, processes: Vec<Arc<Mutex<Function>>>, debugging: bool) -> Self {
        let (dispatch_tx, dispatch_rx, ) = mpsc::sync_channel(1);
        let (output_tx, output_rx) = mpsc::channel();

        execution::looper(dispatch_rx, output_tx);

        RunList {
            state: RunState::new(processes),
            #[cfg(feature = "metrics")]
            metrics: Metrics::new(),
            debugging,
            #[cfg(feature = "debugger")]
            debugger: Debugger::new(client),
            dispatch_tx,
            output_rx,
        }
    }

    fn run(&mut self) {
        let mut display_output;
        let mut restart;

        'outer: loop {
            debug!("Initializing all functions");
            let num_processes = self.state.init();

            if cfg!(feature = "metrics") {
                self.metrics.reset(num_processes);
            }

            debug!("Starting flow execution");
            display_output = false;
            restart = false;

            'inner: while let Some(id) = self.state.next() {
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

                let dispatch = self.dispatch(id);

                let out = if dispatch.impure {
                    debug!("Dispatched on main thread");
                    Ok(execution::execute(dispatch))
                } else {
                    match self.dispatch_tx.send(dispatch) {
                        Ok(_) => debug!("Dispatched on executor thread"),
                        Err(err) => error!("Error dispatching: {}", err)
                    }

                    self.output_rx.recv()
                };

                if let Ok(output) = out {
                    self.update_states(output, display_output);
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
        Given a process id, dispatch it, preparing it for execution.
        Return a tuple with all the information needed to execute it.
    */
    fn dispatch(&mut self, id: usize) -> Dispatch {
        let process_arc = self.state.get(id);
        let process: &mut Function = &mut *process_arc.lock().unwrap();

        let input_values = process.get_input_values();

        self.state.unblock_senders_to(id);
        debug!("Preparing function #{} '{}' for dispatch with inputs: {:?}", id, process.name(), input_values);

        let implementation = process.get_implementation();

        #[cfg(any(feature = "metrics", feature = "debugger"))]
            self.state.increment_dispatches();

        let destinations = process.output_destinations().clone();

        Dispatch { id, implementation, input_values, destinations, impure: process.is_impure() }
    }

    /*
        Take an output produced by a process and modify the runlist accordingly
        If other processs were blocked trying to send to this one - we can now unblock them
        as it has consumed it's inputs and they are free to be sent to again.

        Then take the output and send it to all destination IOs on different processs it should be
        sent to, marking the source process as blocked because those others must consume the output
        if those other processs have all their inputs, then mark them accordingly.
    */
    fn update_states(&mut self, output: Output, display_output: bool) {
        let output_value = output.result.0;
        let source_can_run_again = output.result.1;

        debug!("\tCompleted Function #{}", output.id);
        if cfg!(feature = "debugger") & &display_output {
            self.debugger.client.display(&format!("Completed Function #{}\n", output.id));
        }

        // did it produce any output value
        if let Some(output_v) = output_value {
            debug!("\tProcessing output '{}' from Function #{}", output_v, output.id);

            if cfg!(feature="debugger") && display_output {
                self.debugger.client.display(&format!("\tProduced output {}\n", &output_v));
            }

            for (ref output_route, destination_id, io_number) in output.destinations {
                let destination_arc = self.state.get(destination_id);
                let mut destination = destination_arc.lock().unwrap();
                let output_value = output_v.pointer(&output_route).unwrap();
                debug!("\t\tFunction #{} sent value '{}' via output route '{}' to Function #{} '{}' input :{}",
                       output.id, output_value, output_route, &destination_id, destination.name(), &io_number);
                if cfg!(feature="debugger") && display_output {
                    self.debugger.client.display(
                        &format!("\t\tSending to {}:{}\n", destination_id, io_number));
                }

                #[cfg(feature = "debugger")]
                    self.debugger.watch_data(&mut self.state, output.id, output_route,
                                             &output_value, destination_id, io_number);

                destination.write_input(io_number, output_value.clone());

                #[cfg(feature = "metrics")]
                    self.increment_outputs_sent();

                if destination.input_full(io_number) {
                    self.state.set_blocked_by(destination_id, output.id);
                    #[cfg(feature = "debugger")]
                        self.debugger.check_block(&mut self.state, destination_id, output.id);
                }

                // for the case when a process is sending to itself, delay determining if it should
                // be in the blocked or will_run lists until it has sent all it's other outputs
                // as it might be blocked by another process.
                // Iif not, this will be fixed in the "if source_can_run_again {" block below
                if destination.can_run() && (output.id != destination_id) {
                    self.state.inputs_ready(destination_id);
                }
            }
        }

        // if it wants to run again, and after possibly refreshing any constant inputs, it can
        // (it's inputs are ready) then add back to the Will Run list
        if source_can_run_again {
            let source_arc = self.state.get(output.id);
            let mut source = source_arc.lock().unwrap();

            // refresh any constant inputs it may have
            source.refresh_constant_inputs();

            if source.can_run() {
                self.state.inputs_ready(output.id);
            }
        }

        // remove from the running list
        self.state.done(output.id);
    }

    #[cfg(feature = "metrics")]
    fn print_metrics(&self) {
        println!("\nMetrics: \n {}", self.metrics);
    }

    #[cfg(feature = "metrics")]
    fn increment_outputs_sent(&mut self) {
        self.metrics.outputs_sent += 1;
    }
}

#[test]
fn test_metrics_reset() {
    let mut metrics = Metrics::new();
    metrics.outputs_sent = 10;
    metrics.reset(10);
    assert_eq!(metrics.outputs_sent, 0);
    assert_eq!(metrics.num_processs, 10);
}


use process::Process;
use serde_json::Value as JsonValue;
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
use std::panic;
use std::any::Any;
use std::marker::Send;
use implementation::Implementation;

#[cfg(feature = "metrics")]
pub struct Metrics {
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
        write!(f, "\t\tNumber of Processs: \t{}\n", self.num_processs)?;
        write!(f, "\t\tOutputs sent: \t\t{}\n", self.outputs_sent)?;
        write!(f, "\t\tElapsed time(s): \t{:.*}", 9, elapsed.as_secs() as f64 + elapsed.subsec_nanos() as f64 * 1e-9)
    }
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
pub struct RunList {
    pub state: RunState,

    #[cfg(feature = "metrics")]
    metrics: Metrics,

    debugging: bool,
    #[cfg(feature = "debugger")]
    pub debugger: Debugger,
}

impl RefUnwindSafe for RunList {}

impl UnwindSafe for RunList {}

impl RunList {
    pub fn new(client: &'static DebugClient, processes: Vec<Arc<Mutex<Process>>>, debugging: bool) -> Self {
        RunList {
            state: RunState::new(processes),
            #[cfg(feature = "metrics")]
            metrics: Metrics::new(),
            debugging,
            #[cfg(feature = "debugger")]
            debugger: Debugger::new(client),
        }
    }

    pub fn run(&mut self) {
        let mut display_output;
        let mut restart;

        'outer: loop {
            debug!("Initializing all processes");
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

                let (implementation, input_values, destinations) = self.dispatch(id);

                // TODO these next two would be done sending and receiving on a channel to an executor
                let (source_id, source_input_values, result, destinations) = Self::execute(id, implementation, input_values, destinations);
                self.debug_check(source_id, &source_input_values, &result, display_output);

                self.state.done(id);

                if let Ok((value, run_again)) = result {
                    self.process_output(source_id, destinations, value,
                                        display_output, run_again);
                }
            }

            if !restart {
                if cfg!(feature = "logging") && log_enabled!(Debug) {
                    self.state.print();
                }

                if cfg!(feature = "debugger") && self.debugging {
                    let check = self.debugger.end(&mut self.state);
                    display_output = check.0;
                    restart = check.1;
                }

                if !restart {
                    break 'outer; // We're done!
                }
            }
        }

        debug!("Flow execution ended, no remaining processes ready to run");
    }

    /*
        Given a process id, dispatch it, preparing it for execution.
        Return a tuple with all the information needed to execute it.
    */
    fn dispatch(&mut self, id: usize)
                -> (Arc<Implementation>, Vec<Vec<JsonValue>>, Vec<(String, usize, usize)>) {
        let process_arc = self.state.get(id);
        let process: &mut Process = &mut *process_arc.lock().unwrap();
        debug!("Process #{} '{}' dispatched", id, process.name());

        let input_values = process.get_input_values();

        self.state.unblock_senders_to(id);
        debug!("\tProcess #{} '{}' running with inputs: {:?}", id, process.name(), input_values);

        let implementation = process.get_implementation();

        #[cfg(any(feature = "metrics", feature = "debugger"))]
            self.state.increment_dispatches();

        let destinations = process.output_destinations().clone();

        (implementation, input_values, destinations)
    }

    // TODO send everything to be executed through a channel to the executor
    // TODO this part would go into the executor thread on the other end of a channel
    fn execute(id: usize,
               implementation: Arc<Implementation>,
               input_values: Vec<Vec<JsonValue>>,
               destinations: Vec<(String, usize, usize)>)
               -> (usize, Vec<Vec<JsonValue>>, Result<(Option<JsonValue>, bool), Box<Any + Send>>, Vec<(String, usize, usize)>) {
        let result = panic::catch_unwind(|| { implementation.run(input_values.clone()) });

        // TODO return via a channel, even if there is no output value, so coordinator knows it completed
        return (id, input_values, result, destinations);
    }

    /*
        Using the result of an execution of an implementation, check if we should invoke the
        debugger, either because we need to display the results (e.g. after a step) or because
        the execution panicked.
    */
    fn debug_check(&mut self,
                   id: usize,
                   input_values: &Vec<Vec<JsonValue>>,
                   result: &Result<(Option<JsonValue>, bool), Box<Any + std::marker::Send>>,
                   display_output: bool) {
        match result {
            Ok((_value, _run_again)) => {
                debug!("\tCompleted process: Process #{}", id);
                if cfg!(feature = "debugger") & &display_output {
                    self.debugger.client.display(&format!("Completed process: Process #{}\n", id));
                }
            }
            Err(cause) => {
                if cfg!(feature = "debugger") & &self.debugging {
                    #[cfg(feature = "debugger")]
                        self.debugger.panic(&mut self.state, cause, id, input_values.clone());
                }
            }
        }
    }

    /*
        Take an output produced by a process and modify the runlist accordingly
        If other processs were blocked trying to send to this one - we can now unblock them
        as it has consumed it's inputs and they are free to be sent to again.

        Then take the output and send it to all destination IOs on different processs it should be
        sent to, marking the source process as blocked because those others must consume the output
        if those other processs have all their inputs, then mark them accordingly.
    */
    pub fn process_output(&mut self, source_id: usize, destinations: Vec<(String, usize, usize)>,
                          output_value: Option<JsonValue>, display_output: bool, source_can_run_again: bool) {
        if let Some(output) = output_value {
            debug!("\tProcessing output '{}' from process #{}", output, source_id);

            if cfg!(feature="debugger") && display_output {
                self.debugger.client.display(&format!("\tProduced output {}\n", &output));
            }

            for (ref output_route, destination_id, io_number) in destinations {
                let destination_arc = self.state.get(destination_id);
                let mut destination = destination_arc.lock().unwrap();
                let output_value = output.pointer(&output_route).unwrap();
                debug!("\t\tProcess #{} sent value '{}' via output route '{}' to Process #{} '{}' input :{}",
                       source_id, output_value, output_route, &destination_id, destination.name(), &io_number);
                if cfg!(feature="debugger") && display_output {
                    self.debugger.client.display(
                        &format!("\t\tSending to {}:{}\n", destination_id, io_number));
                }

                #[cfg(feature = "debugger")]
                    self.debugger.watch_data(&mut self.state, source_id, output_route,
                                             &output_value, destination_id, io_number);

                destination.write_input(io_number, output_value.clone());

                #[cfg(feature = "metrics")]
                    self.increment_outputs_sent();

                if destination.input_full(io_number) {
                    self.state.set_blocked_by(destination_id, source_id);
                    #[cfg(feature = "debugger")]
                        self.debugger.check_block(&mut self.state, destination_id, source_id);
                }

                // for the case when a process is sending to itself, delay determining else if
                // it should be in the blocked or will_run lists until it has sent all it's other outputs
                // as it might be blocked by another process - if not it will be fixed lower down in
                // "if source_can_run_again {" blocked
                if destination.can_run() && (source_id != destination_id) {
                    self.state.inputs_ready(destination_id);
                }
            }
        }

        // if it wants to run again, and after possibly refreshing any constant inputs, it can
        // (it's inputs are ready) then add back to the Will Run list
        if source_can_run_again {
            let source_arc = self.state.get(source_id);
            let mut source = source_arc.lock().unwrap();

            // refresh any constant inputs it may have
            source.refresh_constant_inputs();

            if source.can_run() {
                self.state.inputs_ready(source_id);
            }
        }
    }

    #[cfg(feature = "metrics")]
    pub fn print_metrics(&self) {
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


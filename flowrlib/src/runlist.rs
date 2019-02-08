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

#[cfg(feature = "metrics")]
pub struct Metrics {
    num_processs: usize,
    outputs_sent: u32,
    start_time: Instant,
}

#[cfg(feature = "metrics")]
impl Metrics {
    fn new() -> Self {
        let now = Instant::now();
        Metrics {
            num_processs: 0,
            outputs_sent: 0,
            start_time: now,
        }
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
    pub fn new(client: &'static DebugClient, debugging: bool) -> Self {
        #[cfg(feature = "debugger")]
            let debugger = Debugger::new(client);

        let runlist = RunList {
            state: RunState::new(),
            #[cfg(feature = "metrics")]
            metrics: Metrics::new(),
            debugging,
            #[cfg(feature = "debugger")]
            debugger,
        };

        runlist
    }

    /*
        The ìnit' function is responsible for initializing all processs.
        The `init` method on each process is called, which returns a boolean to indicate that it's
        inputs are fulfilled - and this information is added to the RunList to control the readyness of
        the Process to be executed.

        Once all processs have been initialized, the list of processs is stored in the RunList
    */
    pub fn init(&mut self, processs: Vec<Arc<Mutex<Process>>>) {
        debug!("Initializing all processes");
        for process_arc in &processs {
            let mut process = process_arc.lock().unwrap();
            debug!("\tInitializing process #{} '{}'", &process.id(), process.name());
            if process.init() {
                self.state.can_run(process.id());
            }
        }

        if cfg!(feature = "metrics") {
            self.metrics.num_processs = processs.len();
        }

        self.state.set_processes(processs);
    }

    pub fn run(&mut self, processes: Vec<Arc<Mutex<Process>>>) {
        self.init(processes);

        debug!("Starting flow execution");
        let mut display_output = false;

        while let Some(id) = self.state.next() {
            if log_enabled!(Debug) {
                self.state.print();
            }

            if cfg!(feature = "debugger") && self.debugging {
                display_output = self.debugger.check(&self.state, id);
            }

            self.dispatch(id, display_output);
        }

        if cfg!(feature = "logging") && log_enabled!(Debug) {
            self.state.print();
        }

        debug!("Flow execution ended, no remaining processes ready to run");
    }

    /*
        Given a process id, dispatch it
    */
    fn dispatch(&mut self, id: usize, display_output: bool) {
        let process_arc = self.state.get(id);
        let process: &mut Process = &mut *process_arc.lock().unwrap();
        debug!("Process #{} '{}' dispatched", id, process.name());

        let input_values = process.get_input_values();
        self.state.inputs_consumed(id);
        self.state.unblock_senders_to(id);
        debug!("\tProcess #{} '{}' running with inputs: {:?}", id, process.name(), input_values);

        let implementation = process.get_implementation();

        // when a process ends, it can express whether it can be run again or not
        let (value, run_again) = implementation.run(input_values);

        #[cfg(any(feature = "metrics", feature = "debugger"))]
            self.state.increment_dispatches();

        if let Some(val) = value {
            debug!("\tProcess #{} '{}' completed, send output '{}'", id, process.name(), &val);
            if cfg!(feature="debugger") && display_output {
                self.debugger.client.display(
                    &format!("Process #{} '{}' output {}\n", id, process.name(), &val));
            }
            self.process_output(process, val);
        } else {
            debug!("\tProcess #{} '{}' completed, no output", id, process.name());
        }

        // if it wants to run again and it can (inputs ready) then add back to the Can Run list
        if run_again && process.can_run() {
            self.state.can_run(process.id());
        }
    }

    #[cfg(feature = "metrics")]
    pub fn print_metrics(&self) {
        println!("\nMetrics: \n {}", self.metrics);
    }

    /*
        Take an output produced by a process and modify the runlist accordingly
        If other processs were blocked trying to send to this one - we can now unblock them
        as it has consumed it's inputs and they are free to be sent to again.

        Then take the output and send it to all destination IOs on different processs it should be
        sent to, marking the source process as blocked because those others must consume the output
        if those other processs have all their inputs, then mark them accordingly.
    */
    pub fn process_output(&mut self, process: &Process, output: JsonValue) {
        for &(ref output_route, destination_id, io_number) in process.output_destinations() {
            let destination_arc = self.state.get(destination_id);
            let mut destination = destination_arc.lock().unwrap();
            let output_value = output.pointer(&output_route).unwrap();
            debug!("\t\tProcess #{} '{}' sent value '{}' via output '{}' to Process #{} '{}' input #{}",
                   process.id(), process.name(), output_value, output_route, &destination_id,
                   destination.name(), &io_number);

            #[cfg(feature = "debugger")]
                self.debugger.watch_data(&self.state, process.id(), output_route,
                                         &output_value, destination_id, io_number);

            destination.write_input(io_number, output_value.clone());

            #[cfg(feature = "metrics")]
                self.increment_outputs_sent();

            if destination.input_full(io_number) {
                self.state.blocked_by(destination_id, process.id());
            }

            if destination.can_run() {
                self.state.can_run(destination_id);
            }
        }
    }

    #[cfg(feature = "metrics")]
    fn increment_outputs_sent(&mut self) {
        self.metrics.outputs_sent += 1;
    }
}
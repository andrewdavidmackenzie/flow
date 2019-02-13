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
        let mut display_restart;

        'outer: loop { // loop while restart = true
            debug!("Initializing all processes");
            let num_processes = self.state.init();

            if cfg!(feature = "metrics") {
                self.metrics.reset(num_processes);
            }

            debug!("Starting flow execution");
            display_restart = (false, false);

            'inner: while let Some(id) = self.state.next() {
                if log_enabled!(Debug) {
                    self.state.print();
                }

                if cfg!(feature = "debugger") && self.debugging {
                    display_restart = self.debugger.check(&mut self.state, id);

                    if display_restart.1 {
                        break 'inner;
                    }
                }

                self.dispatch(id, display_restart.0);
            }

            if !display_restart.1 {
                if cfg!(feature = "logging") && log_enabled!(Debug) {
                    self.state.print();
                }

                if cfg!(feature = "debugger") && self.debugging {
                    display_restart = self.debugger.end(&mut self.state);
                }

                if !display_restart.1 {
                    break 'outer; // We're done!
                }
            }
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
        #[cfg(feature = "debugger")]
            let debug_values = input_values.clone();

        self.state.inputs_consumed(id);
        self.state.unblock_senders_to(id);
        debug!("\tProcess #{} '{}' running with inputs: {:?}", id, process.name(), input_values);

        let implementation = process.get_implementation();

        #[cfg(any(feature = "metrics", feature = "debugger"))]
            self.state.increment_dispatches();

        // when a process ends, it can express whether it can be run again or not
        let result = panic::catch_unwind(|| {
            implementation.run(input_values)
        });

        match result {
            Ok((value, run_again)) => {
                debug!("\tProcess #{} '{}' completed", id, process.name());
                if cfg!(feature="debugger") && display_output {
                    self.debugger.client.display(
                        &format!("Process #{} '{}' completed\n", id, process.name()));
                }

                if let Some(val) = value {
                    self.process_output(process, val, display_output);
                }

                // if it wants to run again and it can (inputs ready) then add back to the Can Run list
                if run_again & &process.can_run() {
                    self.state.can_run(process.id());
                }
            }
            Err(cause) => {
                if cfg!(feature = "debugger") && self.debugging {
                    #[cfg(feature = "debugger")]
                        self.debugger.panic(&mut self.state, cause, id, process.name(), debug_values);
                }
            }
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
    pub fn process_output(&mut self, process: &Process, output: JsonValue, display_output: bool) {
        debug!("\t\tProduced output '{}'", output);
        if cfg!(feature="debugger") && display_output {
            self.debugger.client.display(
                &format!("\tProduced output {}\n", &output));
        }

        for &(ref output_route, destination_id, io_number) in process.output_destinations() {
            let destination_arc = self.state.get(destination_id);
            let mut destination = destination_arc.lock().unwrap();
            let output_value = output.pointer(&output_route).unwrap();
            debug!("\t\tProcess #{} '{}' sent value '{}' via output '{}' to Process #{} '{}' input #{}",
                   process.id(), process.name(), output_value, output_route, &destination_id,
                   destination.name(), &io_number);
            if cfg!(feature="debugger") && display_output {
                self.debugger.client.display(
                    &format!("\t\tSending to Process #{}\n", destination_id));
            }

            #[cfg(feature = "debugger")]
                self.debugger.watch_data(&mut self.state, process.id(), output_route,
                                         &output_value, destination_id, io_number);

            destination.write_input(io_number, output_value.clone());

            #[cfg(feature = "metrics")]
                self.increment_outputs_sent();

            if destination.input_full(io_number) {
                self.state.blocked_by(destination_id, process.id());
                #[cfg(feature = "debugger")]
                self.debugger.check_block(&mut self.state, destination_id, process.id());
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
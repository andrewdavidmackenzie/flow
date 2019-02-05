use process::Process;
use serde_json::Value as JsonValue;
use std::collections::HashSet;
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
        write!(f, "\t\tElapsed time(s): \t{:.*}\n", 9, elapsed.as_secs() as f64 + elapsed.subsec_nanos() as f64 * 1e-9)
    }
}

pub struct State {
    processs: Vec<Arc<Mutex<Process>>>,
    can_run: HashSet<usize>,
    // can_run: HashSet<process_id>
    blocking: Vec<(usize, usize)>,
    // locking: Vec<(blocking_id, blocked_id)>
    will_run: Vec<usize>,
    // will_run: Vec<process_id>
    dispatches: u32,
}

impl State {
    #[cfg(any(feature = "logging", feature = "debugger"))]
    pub fn print(&self) {
        println!("----------------- Current State --------------------");
        println!("Number of processes: {}", self.processs.len());
        println!("     Dispatch count: {}", self.dispatches);
        println!("            Can Run: {:?}", self.can_run);
        println!("           Blocking: {:?}", self.blocking);
        println!("           Will Run: {:?}", self.will_run);
        println!("----------------------------------------------------");
    }

    fn get(&self, id: usize) -> Arc<Mutex<Process>> {
        self.processs[id].clone()
    }

    // Return the id of the next process ready to be run, if there is one
    pub fn next(&mut self) -> Option<usize> {
        if self.will_run.is_empty() {
            return None;
        }

        Some(self.will_run.remove(0))
    }

    // TODO ADM optimize this by also having a flag in the process?
    // Or use the blocked_id as a key to a HashSet?
    // See if there is any tuple in the vector where the second (blocked_id) is the one we're after
    fn is_blocked(&self, id: usize) -> bool {
        for &(_blocking_id, blocked_id) in &self.blocking {
            if blocked_id == id {
                return true;
            }
        }
        false
    }

    // save the fact that a particular Process's inputs are now satisfied and so it maybe ready
    // to run (if not blocked sending on it's output)
    pub fn can_run(&mut self, id: usize) {
        debug!("\t\t\tProcess #{} inputs are ready", id);
        self.can_run.insert(id);

        if !self.is_blocked(id) {
            debug!("\t\t\tProcess #{} not blocked on output, so added to end of 'Will Run' list", id);
            self.will_run.push(id);
        }
    }

    pub fn dispatches(&self) -> u32 {
        self.dispatches
    }

    // unblock all processs that were blocked trying to send to blocker_id by removing all entries
    // in the list where the first value (blocking_id) matches the destination_id
    // when each is unblocked on output, if it's inputs are satisfied, then it is ready to be run
    // again, so put it on the ready queue
    pub fn unblock_senders_to(&mut self, blocker_id: usize) {
        if !self.blocking.is_empty() {
            let mut unblocked_list = vec!();

            for &(blocking_id, blocked_id) in &self.blocking {
                if blocking_id == blocker_id {
                    debug!("\t\tProcess #{} <-- #{} - unblocked", blocking_id, blocked_id);
                    unblocked_list.push(blocked_id);
                }
            }

            // when done remove all entries from the blocking list where it was this blocker_id
            self.blocking.retain(|&(blocking_id, _blocked_id)| blocking_id != blocker_id);

            // see if the ones unblocked should be made ready. Note, they could be blocked on others not the
            // one that unblocked.
            for unblocked in unblocked_list {
                if self.can_run.contains(&unblocked) && !self.is_blocked(unblocked) {
                    debug!("\t\t\tProcess #{} has inputs ready, so added to end of 'Will Run' list", unblocked);
                    self.will_run.push(unblocked);
                }
            }
        }
    }

    // Save the fact that the process 'blocked_id' is blocked on it's output by 'blocking_id'
    pub fn blocked_by(&mut self, blocking_id: usize, blocked_id: usize) {
        // avoid deadlocks by a process blocking itself
        if blocked_id != blocking_id {
            debug!("\t\t\tProcess #{} <-- Process #{} blocked",
                   &blocking_id, &blocked_id);
            self.blocking.push((blocking_id, blocked_id));
        }
    }

    // when a process consumes it's inputs, then take if off the list of processs with inputs ready
    pub fn inputs_consumed(&mut self, id: usize) {
        debug!("\tProcess #{} consumed its inputs, so removed from 'Can Run' list", id);
        self.can_run.remove(&id);
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
    pub state: State,

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
            state: State {
                processs: Vec::<Arc<Mutex<Process>>>::new(),
                can_run: HashSet::<usize>::new(),
                blocking: Vec::<(usize, usize)>::new(),
                will_run: Vec::<usize>::new(),
                #[cfg(feature = "debugger")]
                dispatches: 0,
            },
            #[cfg(feature = "metrics")]
            metrics: Metrics::new(),
            debugging,
            #[cfg(feature = "debugger")]
            debugger,
        };

        runlist
    }

    pub fn run(&mut self) {
        debug!("Starting flow execution");

        while let Some(id) = self.state.next() {
            if log_enabled!(Debug) {
                self.state.print();
            }

            if cfg!(feature = "debugger") && self.debugging {
                #[cfg(feature = "debugger")]
                    self.debugger.check(&self.state);
            }

            self.dispatch(id);
        }

        if cfg!(feature = "logging") && log_enabled!(Debug) {
            self.state.print();
        }

        debug!("Flow execution ended, no remaining processes ready to run");
    }

    /*
        Given a process id, start running it
    */
    fn dispatch(&mut self, id: usize) {
        let process_arc = self.state.get(id);
        let process: &mut Process = &mut *process_arc.lock().unwrap();
        debug!("Process #{} '{}' dispatched", id, process.name());

        let input_values = process.get_input_values();
        self.state.inputs_consumed(id);
        self.state.unblock_senders_to(id);
        debug!("\tProcess #{} '{}' running with inputs: {:?}", id, process.name(), input_values);

        let implementation = process.get_implementation();

        // when a process ends, it can express whether it can run again or not
        let (value, run_again) = implementation.run(input_values);

        #[cfg(any(feature = "metrics", feature = "debugger"))]
            self.increment_dispatches();

        if let Some(val) = value {
            debug!("\tProcess #{} '{}' completed, send output '{}'", id, process.name(), &val);
            self.process_output(process, val);
        } else {
            debug!("\tProcess #{} '{}' completed, no output", id, process.name());
        }
        // if it wants to run again and it can (inputs ready) then add back to the Can Run list
        if run_again && process.can_run() {
            self.state.can_run(process.id());
        }
    }

    pub fn set_processes(&mut self, processs: Vec<Arc<Mutex<Process>>>) {
        #[cfg(feature = "metrics")]
        self.set_num_processes(processs.len());

        self.state.processs = processs;
    }

    #[cfg(feature = "metrics")]
    pub fn print_metrics(&self) {
        println!("\nMetrics: \n {}", self.metrics);
    }

    #[cfg(feature = "metrics")]
    fn set_num_processes(&mut self, num: usize) {
        self.metrics.num_processs = num;
    }

    #[cfg(any(feature = "metrics", feature = "debugger"))]
    fn increment_dispatches(&mut self) {}

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
            let destination_arc = Arc::clone(&self.state.processs[destination_id]);
            let mut destination = destination_arc.lock().unwrap();
            let output_value = output.pointer(&output_route).unwrap();
            debug!("\t\tProcess #{} '{}' sent value '{}' via output '{}' to Process #{} '{}' input #{}",
                   process.id(), process.name(), output_value, output_route, &destination_id,
                   destination.name(), &io_number);
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

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use std::io;
    use std::io::Write;

    use super::Process;
    use super::RunList;
    use debug_client::DebugClient;

    struct CLIDebugClient {}

    impl DebugClient for CLIDebugClient {
        fn display(&self, output: &str) {
            print!("{}", output);
            io::stdout().flush().unwrap();
        }

        fn read_input(&self, input: &mut String) -> io::Result<usize> {
            io::stdin().read_line(input)
        }
    }

    const CLI_DEBUG_CLIENT: &DebugClient = &CLIDebugClient {};

    fn test_processs<'a>() -> Vec<Arc<Mutex<Process>>> {
        let p0 = Arc::new(Mutex::new(
            Process::new("p0", // name
                         false,// static value
                         "/test".to_string(),
                         vec!(), // input depths array
                         0,    // id
                         None,
                         vec!(("".to_string(), 1, 0), ("".to_string(), 1, 0)), // destinations
            )));    // implementation
        let p1 = Arc::new(Mutex::new(Process::new("p1",
                                                  false,// static value
                                                  "/test".to_string(),
                                                  vec!(1), // input depths array
                                                  1,    // id
                                                  None,
                                                  vec!(),
        )));
        let p2 = Arc::new(Mutex::new(Process::new("p2",
                                                  false,// static value
                                                  "/test".to_string(),
                                                  vec!(1), // input depths array
                                                  2,    // id
                                                  None,
                                                  vec!(),
        )));
        vec!(p0, p1, p2)
    }

    #[test]
    fn blocked_works() {
        let processs = test_processs();
        let mut runs = RunList::new(CLI_DEBUG_CLIENT, false);
        runs.set_processes(processs);

        // Indicate that 0 is blocked by 1
        runs.blocked_by(1, 0);
        assert!(runs.is_blocked(0));
    }

    #[test]
    fn get_works() {
        let processs = test_processs();
        let mut runs = RunList::new(CLI_DEBUG_CLIENT, false);
        runs.set_processes(processs);
        let got_arc = runs.get(1);
        let got = got_arc.lock().unwrap();
        assert_eq!(got.id(), 1)
    }

    #[test]
    fn no_next_if_none_ready() {
        let processs = test_processs();
        let mut runs = RunList::new(CLI_DEBUG_CLIENT, false);
        runs.set_processes(processs);

        assert!(runs.next().is_none());
    }

    #[test]
    fn inputs_ready_makes_ready() {
        let processs = test_processs();
        let mut runs = RunList::new(CLI_DEBUG_CLIENT, false);
        runs.set_processes(processs);

        // Indicate that 0 has all it's inputs read
        runs.can_run(0);

        assert_eq!(runs.next().unwrap(), 0);
    }

    #[test]
    fn blocked_is_not_ready() {
        let processs = test_processs();
        let mut runs = RunList::new(CLI_DEBUG_CLIENT, false);
        runs.set_processes(processs);

        // Indicate that 0 is blocked by 1
        runs.blocked_by(1, 0);

        // Indicate that 0 has all it's inputs read
        runs.can_run(0);

        match runs.next() {
            None => assert!(true),
            Some(_) => assert!(false)
        }
    }

    #[test]
    fn unblocking_makes_ready() {
        let processs = test_processs();
        let mut runs = RunList::new(CLI_DEBUG_CLIENT, false);
        runs.set_processes(processs);

        // Indicate that 0 is blocked by 1
        runs.blocked_by(1, 0);

        // Indicate that 0 has all it's inputs read
        runs.can_run(0);

        assert_eq!(runs.next(), None);

        // now unblock 0 by 1
        runs.unblock_senders_to(1);

        // Now process with id 0 should be ready and served up by next
        assert_eq!(runs.next(), Some(0));
    }

    #[test]
    fn unblocking_doubly_blocked_process_not_ready() {
        let processs = test_processs();
        let mut runs = RunList::new(CLI_DEBUG_CLIENT, false);
        runs.set_processes(processs);

        // Indicate that 0 is blocked by 1 and 2
        runs.blocked_by(1, 0);
        runs.blocked_by(2, 0);

        // Indicate that 0 has all it's inputs read
        runs.can_run(0);

        assert_eq!(runs.next(), None);

        // now unblock 0 by 1
        runs.unblock_senders_to(1);

        // Now process with id 0 should still not be ready as still blocked on 2
        assert_eq!(runs.next(), None);
    }
}
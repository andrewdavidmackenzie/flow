use process::Process;
use serde_json::Value as JsonValue;
use std::collections::HashSet;
use std::fmt;
use std::panic::RefUnwindSafe;
use std::panic::UnwindSafe;
use std::sync::{Arc, Mutex};
use std::time::Instant;

pub struct Metrics {
    num_processs: usize,
    invocations: u32,
    outputs_sent: u32,
    start_time: Instant,
}

impl Metrics {
    fn new() -> Self {
        let now = Instant::now();
        Metrics {
            num_processs: 0,
            invocations: 0,
            outputs_sent: 0,
            start_time: now,
        }
    }
}

impl fmt::Display for Metrics {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let elapsed = self.start_time.elapsed();
        write!(f, "\t\tNumber of Processs: \t{}\n", self.num_processs)?;
        write!(f, "\t\tProcess invocations: \t{}\n", self.invocations)?;
        write!(f, "\t\tOutputs sent: \t\t{}\n", self.outputs_sent)?;
        write!(f, "\t\tElapsed time(s): \t{:.*}\n", 9, elapsed.as_secs() as f64 + elapsed.subsec_nanos() as f64 * 1e-9)
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
pub struct RunList<'a> {
    processs: Vec<Arc<Mutex<Process<'a>>>>,
    can_run: HashSet<usize>,
    // process_id
    blocking: Vec<(usize, usize)>,
    // blocking_id, blocked_id
    will_run: Vec<usize>,
    // process_id
    metrics: Metrics,
}

impl<'a> RefUnwindSafe for RunList<'a> {}

impl<'a> UnwindSafe for RunList<'a> {}

impl<'a> RunList<'a> {
    pub fn new() -> Self {
        RunList {
            processs: Vec::<Arc<Mutex<Process>>>::new(),
            can_run: HashSet::<usize>::new(),
            blocking: Vec::<(usize, usize)>::new(),
            will_run: Vec::<usize>::new(),
            metrics: Metrics::new(),
        }
    }

    pub fn debug(&self) {
        debug!("Dispatch count: {}", self.metrics.invocations);
        debug!("       Can Run: {:?}", self.can_run);
        debug!("      Blocking: {:?}", self.blocking);
        debug!("      Will Run: {:?}", self.will_run);
        debug!("-------------------------------------");
    }

    pub fn end(&self) {
        debug!("Metrics: \n {}", self.metrics);
    }

    pub fn set_processs(&mut self, processs: Vec<Arc<Mutex<Process<'a>>>>) {
        self.processs = processs;
        self.metrics.num_processs = self.processs.len();
    }

    pub fn get(&self, id: usize) -> Arc<Mutex<Process<'a>>> {
        self.processs[id].clone()
    }

    // Return the id of the next process ready to be run, if there is one
    pub fn next(&mut self) -> Option<usize> {
        if self.will_run.is_empty() {
            return None;
        }

        self.metrics.invocations += 1;
        Some(self.will_run.remove(0))
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

    // when a process consumes it's inputs, then take if off the list of processs with inputs ready
    pub fn inputs_consumed(&mut self, id: usize) {
        debug!("\tProcess #{} consumed its inputs, removing from the 'Can Run' list", id);
        self.can_run.remove(&id);
    }

    /*
        Take an output produced by a process and modify the runlist accordingly
        If other processs were blocked trying to send to this one - we can now unblock them
        as it has consumed it's inputs and they are free to be sent to again.

        Then take the output and send it to all destination IOs on different processs it should be
        sent to, marking the source process as blocked because those others must consume the output
        if those other processs have all their inputs, then mark them accordingly.
    */
    pub fn send_output(&mut self, process: &Process, output: JsonValue) {
        for &(ref output_route, destination_id, io_number) in process.output_destinations() {
            let destination_arc = Arc::clone(&self.processs[destination_id]);
            let mut destination = destination_arc.lock().unwrap();
            let output_value = output.pointer(&output_route).unwrap();
            debug!("\t\tProcess #{} '{}{}' sending output '{}' to Process #{} '{}' input #{}",
                   process.id(), process.name(), output_route, output_value, &destination_id,
                   destination.name(), &io_number);
            destination.write_input(io_number, output_value.clone());
            self.metrics.outputs_sent += 1;
            if destination.input_full(io_number) {
                self.blocked_by(destination_id, process.id());
            }

            if destination.can_run() {
                self.can_run(destination_id);
            }
        }
    }

    // Save the fact that the process 'blocked_id' is blocked on it's output by 'blocking_id'
    pub fn blocked_by(&mut self, blocking_id: usize, blocked_id: usize) {
        // avoid deadlocks by a process blocking itself
        if blocked_id != blocking_id {
            debug!("\t\t\tProcess #{} is now blocked on output by Process #{}", &blocked_id, &blocking_id);
            self.blocking.push((blocking_id, blocked_id));
        }
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
                    debug!("\t\tProcess #{} <-- #{} - block removed", blocking_id, blocked_id);
                    unblocked_list.push(blocked_id);
                }
            }

            // when done remove all entries from the blocking list where it was this blocker_id
            self.blocking.retain(|&(blocking_id, _blocked_id)| blocking_id != blocker_id);

            // see if the ones unblocked should be made ready. Note, they could be blocked on others not the
            // one that unblocked.
            for unblocked in unblocked_list {
                if self.can_run.contains(&unblocked) && !self.is_blocked(unblocked) {
                    debug!("\t\t\tProcess #{} was unblocked and it inputs are ready, so added to end of 'Will Run' list", unblocked);
                    self.will_run.push(unblocked);
                }
            }
        }
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
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::Process;
    use super::RunList;

    fn test_processs<'a>() -> Vec<Arc<Mutex<Process<'a>>>> {
        let p0 = Arc::new(Mutex::new(
            Process::new("p0", // name
                         0,    // number_of_inputs
                         false,// static value
                         "/test".to_string(),
                         vec!(), // input depths array
                         0,    // id
                         None,
                         vec!(("".to_string(), 1, 0), ("".to_string(), 1, 0)), // destinations
            )));    // implementation
        let p1 = Arc::new(Mutex::new(Process::new("p1",
                                                  1,
                                                  false,// static value
                                                  "/test".to_string(),
                                                  vec!(1), // input depths array
                                                  1,    // id
                                                  None,
                                                  vec!(),
        )));
        let p2 = Arc::new(Mutex::new(Process::new("p2",
                                                  1,
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
        let mut runs = RunList::new();
        runs.set_processs(processs);

        // Indicate that 0 is blocked by 1
        runs.blocked_by(1, 0);
        assert!(runs.is_blocked(0));
    }

    #[test]
    fn get_works() {
        let processs = test_processs();
        let mut runs = RunList::new();
        runs.set_processs(processs);
        let got_arc = runs.get(1);
        let got = got_arc.lock().unwrap();
        assert_eq!(got.id(), 1)
    }

    #[test]
    fn no_next_if_none_ready() {
        let processs = test_processs();
        let mut runs = RunList::new();
        runs.set_processs(processs);

        assert!(runs.next().is_none());
    }

    #[test]
    fn inputs_ready_makes_ready() {
        let processs = test_processs();
        let mut runs = RunList::new();
        runs.set_processs(processs);

        // Indicate that 0 has all it's inputs read
        runs.can_run(0);

        assert_eq!(runs.next().unwrap(), 0);
    }

    #[test]
    fn blocked_is_not_ready() {
        let processs = test_processs();
        let mut runs = RunList::new();
        runs.set_processs(processs);

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
        let mut runs = RunList::new();
        runs.set_processs(processs);

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
        let mut runs = RunList::new();
        runs.set_processs(processs);

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
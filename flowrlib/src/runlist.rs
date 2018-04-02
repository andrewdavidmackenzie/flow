use runnable::Runnable;
use std::sync::{Arc, Mutex};
use std::collections::HashSet;
use std::fmt;
use std::time::Instant;
use serde_json::Value as JsonValue;
use std::panic::RefUnwindSafe;
use std::panic::UnwindSafe;

pub struct Metrics {
    num_runnables: usize,
    invocations: u32,
    outputs_sent: u32,
    start_time: Instant,
}

impl Metrics {
    fn new() -> Self {
        let now = Instant::now();
        Metrics {
            num_runnables: 0,
            invocations: 0,
            outputs_sent: 0,
            start_time: now,
        }
    }
}

impl fmt::Display for Metrics {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let elapsed = self.start_time.elapsed();
        write!(f, "\t\tNumber of Runnables: \t{}\n", self.num_runnables)?;
        write!(f, "\t\tRunnable invocations: \t{}\n", self.invocations)?;
        write!(f, "\t\tOutputs sent: \t\t{}\n", self.outputs_sent)?;
        write!(f, "\t\tElapsed time(s): \t{:.*}\n", 9, elapsed.as_secs() as f64 + elapsed.subsec_nanos() as f64 * 1e-9)
    }
}

/*
    RunList is a structure that maintains the state of all the runnables in the currently
    executing flow.

    A runnable maybe blocking multiple others trying to send data to it.
    Those others maybe blocked trying to send to multiple different runnables.

    runnables:
    A list of all the runnables that could be executed at some point.

    inputs_satisfied:
    A list of runnables who's inputs are satisfied.

    blocking:
    A list of tuples of runnable ids where first id is id of the runnable data is trying to be sent
    to, and the second id is the id of the runnable trying to send data.

    ready:
    A list of Runnables who are ready to be run, they have their inputs satisfied and they are not
    blocked on the output (so their output can be produced).
*/
pub struct RunList {
    runnables: Vec<Arc<Mutex<Runnable>>>,
    inputs_ready: HashSet<usize>,
    // runnable_id
    blocking: Vec<(usize, usize)>,
    // blocking_id, blocked_id
    ready: Vec<usize>,
    // runnable_id
    metrics: Metrics,
}

impl RefUnwindSafe for RunList {}

impl UnwindSafe for RunList {}

impl RunList {
    pub fn new() -> Self {
        RunList {
            runnables: Vec::<Arc<Mutex<Runnable>>>::new(),
            inputs_ready: HashSet::<usize>::new(),
            blocking: Vec::<(usize, usize)>::new(),
            ready: Vec::<usize>::new(),
            metrics: Metrics::new(),
        }
    }

    pub fn end(&self) {
        debug!("Metrics: \n {}", self.metrics);
    }

    pub fn set_runnables(&mut self, runnables: Vec<Arc<Mutex<Runnable>>>) {
        self.runnables = runnables;
        self.metrics.num_runnables = self.runnables.len();
    }

    pub fn get(&self, id: usize) -> Arc<Mutex<Runnable>> {
        self.runnables[id].clone()
    }

    // Return the id of the next runnable ready to be run, if there is one
    pub fn next(&mut self) -> Option<usize> {
        if self.ready.is_empty() {
            return None;
        }

        self.metrics.invocations += 1;
        Some(self.ready.remove(0))
    }

    // save the fact that a particular Runnable's inputs are now satisfied and so it maybe ready
    // to run (if not blocked sending on it's output)
    pub fn inputs_ready(&mut self, id: usize) {
        debug!("\t\tRunnable #{} inputs are ready", id);
        self.inputs_ready.insert(id);

        if !self.is_blocked(id) {
            debug!("\t\tRunnable #{} not blocked on output, so added to end of READY list", id);
            self.ready.push(id);
        }
    }

    // when a runnable consumes it's inputs, then take if off the list of runnables with inputs ready
    pub fn inputs_consumed(&mut self, id: usize) {
        self.inputs_ready.remove(&id);
    }

    /*
        Take an output produced by a runnable and modify the runlist accordingly
        If other runnables were blocked trying to send to this one - we can now unblock them
        as it has consumed it's inputs and they are free to be sent to again.

        Then take the output and send it to all destination IOs on different runnables it should be
        sent to, marking the source runnable as blocked because those others must consume the output
        if those other runnables have all their inputs, then mark them accordingly.
    */
    pub fn send_output(&mut self, runnable: &Runnable, output: JsonValue) {
        for &(output_route, destination_id, io_number) in runnable.output_destinations() {
            let destination_arc = Arc::clone(&self.runnables[destination_id]);
            let mut destination = destination_arc.lock().unwrap();
            let output_value = output.pointer(output_route).unwrap();
            destination.write_input(io_number, output_value.clone());
            self.metrics.outputs_sent += 1;
            debug!("\tRunnable #{} '{}/{}' sent output '{}' to Runnable #{} '{}' input #{}",
                   runnable.id(), runnable.name(), output_route, output_value, &destination_id,
                   destination.name(), &io_number);
            if destination.input_full(io_number) {
                self.blocked_by(destination_id, runnable.id());
            }

            if destination.inputs_full() {
                self.inputs_ready(destination_id);
            }
        }
    }

    // Save the fact that the runnable 'blocked_id' is blocked on it's output by 'blocking_id'
    pub fn blocked_by(&mut self, blocking_id: usize, blocked_id: usize) {
        debug!("\t\tRunnable #{} is now blocked on output by Runnable #{}", &blocked_id, &blocking_id);
        self.blocking.push((blocking_id, blocked_id));
    }

    // unblock all runnables that were blocked trying to send to blocker_id by removing all entries
    // in the list where the first value (blocking_id) matches the destination_id
    // when each is unblocked on output, if it's inputs are satisfied, then it is ready to be run
    // again, so put it on the ready queue
    pub fn unblock_senders_to(&mut self, blocker_id: usize) {
        if !self.blocking.is_empty() {
            for &(blocking_id, blocked_id) in &self.blocking {
                if blocking_id == blocker_id {
                    debug!("\t\tRunnable #{} was blocked sending to #{}", blocked_id, blocking_id);

                    if self.inputs_ready.contains(&blocked_id) {
                        debug!("\t\tRunnable #{} inputs are ready, so added to end of READY list", blocked_id);
                        self.ready.push(blocked_id);
                    }
                }
            }

            // when done remove all entries from the blocking list where it was this blocker_id runnable that was blocking others
            self.blocking.retain(|&(blocking_id, _blocked_id)| blocking_id != blocker_id);
        }
    }

    // TODO ADM optimize this by also having a flag in the runnable?
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
    use super::RunList;
    use super::Runnable;
    use std::sync::{Arc, Mutex};
    use serde_json;
    use serde_json::Value as JsonValue;
    use super::super::implementation::Implementation;

    struct TestImplementation;

    impl Implementation for TestImplementation {
        fn run(&self, runnable: &Runnable, inputs: Vec<Vec<JsonValue>>, run_list: &mut RunList) {
            run_list.send_output(runnable, inputs.get(0).unwrap().get(0).unwrap().clone())
        }
    }

    struct TestRunnable {
        id: usize,
        number_of_inputs: usize,
        destinations: Vec<(&'static str, usize, usize)>,
        implementation: Box<Implementation>,
    }

    impl TestRunnable {
        fn new(id: usize, number_of_inputs: usize, destinations: Vec<(&'static str, usize, usize)>) -> TestRunnable {
            TestRunnable {
                id,
                number_of_inputs,
                destinations,
                implementation: Box::new(TestImplementation),
            }
        }
    }

    impl Runnable for TestRunnable {
        fn name(&self) -> &str { "TestRunnable" }
        fn number_of_inputs(&self) -> usize { self.number_of_inputs }
        fn id(&self) -> usize { self.id }
        fn init(&mut self) -> bool { false }
        fn write_input(&mut self, _input_number: usize, _new_value: JsonValue) {}
        fn input_full(&self, _input_number: usize) -> bool { true }
        fn inputs_full(&self) -> bool { true }
        fn get_inputs(&mut self) -> Vec<Vec<JsonValue>> {
            vec!(vec!(serde_json::from_str("Input").unwrap()))
        }
        fn output_destinations(&self) -> &Vec<(&'static str, usize, usize)> { &self.destinations }
        fn implementation(&self) -> &Box<Implementation> { &self.implementation }
    }

    fn test_runnables() -> Vec<Arc<Mutex<Runnable>>> {
        let r0 = Arc::new(Mutex::new(TestRunnable::new(0, 0, vec!(("", 1, 0)))));
        let r1 = Arc::new(Mutex::new(TestRunnable::new(1, 1, vec!())));
        vec!(r0, r1)
    }

    #[test]
    fn blocked_works() {
        let runnables = test_runnables();
        let mut runs = RunList::new();
        runs.set_runnables(runnables);

        // Indicate that 0 is blocked by 1
        runs.blocked_by(1, 0);
        assert!(runs.is_blocked(0));
    }

    #[test]
    fn get_works() {
        let runnables = test_runnables();
        let mut runs = RunList::new();
        runs.set_runnables(runnables);
        let got_arc = runs.get(1);
        let got = got_arc.lock().unwrap();
        assert_eq!(got.id(), 1)
    }

    #[test]
    fn no_next_if_none_ready() {
        let runnables = test_runnables();
        let mut runs = RunList::new();
        runs.set_runnables(runnables);

        assert!(runs.next().is_none());
    }

    #[test]
    fn inputs_ready_makes_ready() {
        let runnables = test_runnables();
        let mut runs = RunList::new();
        runs.set_runnables(runnables);

        // Indicate that 0 has all it's inputs read
        runs.inputs_ready(0);

        assert_eq!(runs.next().unwrap(), 0);
    }

    #[test]
    fn blocked_is_not_ready() {
        let runnables = test_runnables();
        let mut runs = RunList::new();
        runs.set_runnables(runnables);

        // Indicate that 0 is blocked by 1
        runs.blocked_by(1, 0);

        // Indicate that 0 has all it's inputs read
        runs.inputs_ready(0);

        match runs.next() {
            None => assert!(true),
            Some(_) => assert!(false)
        }
    }

    #[test]
    fn unblocking_makes_ready() {
        let runnables = test_runnables();
        let mut runs = RunList::new();
        runs.set_runnables(runnables);

        // Indicate that 0 is blocked by 1
        runs.blocked_by(1, 0);

        // Indicate that 0 has all it's inputs read
        runs.inputs_ready(0);

        assert_eq!(runs.next(), None);

        // now unblock 0 by 1
        runs.unblock_senders_to(1);

        // Now runnable with id 0 should be ready and served up by next
        assert_eq!(runs.next(), Some(0));
    }
}
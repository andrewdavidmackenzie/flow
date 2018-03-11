use runnable::Runnable;
use std::sync::{Arc, Mutex};
use std::collections::HashSet;

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
    inputs_satisfied: HashSet<usize>,
    blocking: Vec<(usize, usize)>,
    ready: Vec<usize>,
}

impl RunList {
    pub fn new() -> Self {
        RunList {
            runnables: Vec::<Arc<Mutex<Runnable>>>::new(),
            inputs_satisfied: HashSet::<usize>::new(),
            blocking: Vec::<(usize, usize)>::new(),
            ready: Vec::<usize>::new(),
        }
    }

    pub fn set_runnables(&mut self, runnables: Vec<Arc<Mutex<Runnable>>>) {
        self.runnables = runnables;
    }

    pub fn get(&self, id: usize) -> Arc<Mutex<Runnable>> {
        self.runnables[id].clone()
    }

    // Return the next runnable ready to be run, if there is one
    pub fn next(&mut self) -> Option<usize> {
        if self.ready.is_empty() {
            return None;
        }

        let id = self.ready.remove(0);
        debug!("Next ready runnable in runlist: {}", id);
        Some(id)
    }

    // save the fact that a particular Runnable's inputs are now satisfied and so it maybe ready
    // to run (if not blocked sending on it's output)
    pub fn inputs_ready(&mut self, id: usize) {
        debug!("Runnable #{}'s inputs are all ready", id);

        if self.is_blocked(id) {
            debug!("Runnable #{} is blocked on output", id);
            self.inputs_satisfied.insert(id);
        } else {
            debug!("Runnable #{} marked as ready", id);
            self.ready.push(id);
        }
    }

    /*
        Take an output produced by a runnable and modify the runlist accordingly
        If other runnables were blocked trying to send to this one - we can now unblock them
        as it has consumed it's inputs and they are free to be sent to again.

        Then take the output and send it to all destination IOs on different runnables it should be
        sent to, marking the source runnable as blocked because those others must consume the output
        if those other runnables have all their inputs, then mark them accordingly.
    */
    pub fn process_output(&mut self, runnable: &Runnable, output: Option<String>) {
        self.unblock_by(runnable.id());

        for &(destination_id, io_number) in runnable.output_destinations() {
            let destination_arc = Arc::clone(&self.runnables[destination_id]);
            let mut destination = destination_arc.lock().unwrap();
            debug!("Sending output '{:?}' from #{} to #{} input #{}",
                   &output, runnable.id(), &destination_id, &io_number);
            self.blocked_by(destination_id, runnable.id());
            destination.write_input(io_number, output.clone());
            if destination.inputs_satisfied() {
                self.inputs_ready(destination_id);
            }
        }
    }

    // Save the fact that the runnable 'blocked_id' is blocked on it's output by 'blocking_id'
    pub fn blocked_by(&mut self, blocking_id: usize, blocked_id: usize) {
        debug!("Runnable #{} is blocked on output by runnable #{}", &blocked_id, &blocking_id);
        self.blocking.push((blocking_id, blocked_id));
    }

    // unblock all runnables that were blocked trying to send to destination_id by removing all entries
    // in the list where the first value (blocking_id) matches the destination_id
    // when each is unblocked on output, if it's inputs are satisfied, then it is ready to be run
    // so put it on the ready queue
    pub fn unblock_by(&mut self, destination_id: usize) {
        debug!("Unblocking runnables blocked on #{}", &destination_id);
        for &(blocking_id, blocked_id) in &self.blocking {
            if blocking_id == destination_id {
                if self.inputs_satisfied.remove(&blocked_id) {
                    self.ready.push(blocked_id);
                }
            }
        }

        self.blocking.retain(|&(blocking_id, _blocked_id)| blocking_id != destination_id);
    }

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

    struct TestRunnable {
        id: usize,
        destinations: Vec<(usize, usize)>
    }

    impl TestRunnable {
        fn new(id: usize) -> TestRunnable {
            TestRunnable {
                id,
                destinations: vec!((1, 0))
            }
        }
    }

    impl Runnable for TestRunnable {
        fn name(&self) -> &str { "TestRunnable"}
        fn number_of_inputs(&self) -> usize { 1 }
        fn id(&self) -> usize { self.id }
        fn init(&mut self) -> bool { false }
        fn write_input(&mut self, _input_number: usize, _new_value: Option<String>) {}
        fn inputs_satisfied(&self) -> bool { false }
        fn run(&mut self) -> Option<String> { Some("Output".to_string()) }
        fn output_destinations(&self) -> &Vec<(usize, usize)> { &self.destinations }
    }

    fn test_runnables() -> Vec<Arc<Mutex<Runnable>>> {
        let r0 = Arc::new(Mutex::new(TestRunnable::new(0)));
        let r1 = Arc::new(Mutex::new(TestRunnable::new(1)));
        let mut runnables: Vec<Arc<Mutex<Runnable>>> = Vec::new();
        runnables.push(r0);
        runnables.push(r1);
        runnables
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

        assert!(runs.next().is_none());

        // now unblock 0 by 1
        runs.unblock_by(1);

        assert_eq!(runs.next().unwrap(), 0);
    }
}
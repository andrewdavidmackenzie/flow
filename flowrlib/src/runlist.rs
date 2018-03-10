use runnable::Runnable;
use std::sync::{Arc, Mutex};
use std::collections::HashSet;

/*
    inputs_satisfied:
    A list of runnables who's inputs are satisfied.

    blocked:
    A list of tuples of runnable ids where first runnable_id is where data is trying
    to be sent to, and the second runnable_id is the runnable trying to send to it.
    Vec<(runnable_to_send_to, runnable_that_is_blocked_on_output)>

    Note that a runnable maybe blocking multiple others trying to send to it.
    Those others maybe blocked trying to send to multiple.
    So, when a runnable is run, we remove all entries that depend on it.
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
            runnables: vec!(),
            inputs_satisfied: HashSet::<usize>::new(),
            blocking: Vec::<(usize, usize)>::new(),
            ready: Vec::<usize>::new(),
        }
    }

    pub fn set_runnables(&mut self, runnables: Vec<Arc<Mutex<Runnable>>>) {
        self.runnables = runnables;
    }

    // Get a runnable from the runnable id
    // TODO get this to return a mutable reference and avoid cloning
    pub fn get(&self, id: usize) -> Arc<Mutex<Runnable>> {
        self.runnables[id].clone()
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

    // Return the next runnable at the head of the ready list if there is one
    pub fn next(&mut self) -> Option<Arc<Mutex<Runnable>>> {
        if self.ready.len() == 0 {
            return None;
        }

        debug!("Ready list: {:?}", self.ready);

        let id = self.ready.remove(0);
        // TODO try to return a reference and avoid this clone
        Some(self.runnables[id].clone())
    }

    // Save the fact that the runnable 'blocked_id' is blocked on it's output by 'blocking_id'
    pub fn blocked_by(&mut self, blocking_id: usize, blocked_id: usize) {
        info!("Runnable #{} is blocking runnable #{}", &blocking_id, &blocked_id);
        self.blocking.push((blocking_id, blocked_id));
    }

    // unblock all runnables that were blocked trying to send to destination_id by removing all entries
    // in the list where the first value (blocking_id) matches the destination_id
    pub fn unblock_by(&mut self, destination_id: usize) {
        info!("Unblocking runnables blocked on #{}", &destination_id);
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
//    use std::fmt;

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
/*
    impl fmt::Display for TestRunnable {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "\tid: {}\n", self.id).unwrap();
            Ok(())
        }
    }*/

    impl Runnable for TestRunnable {
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
    fn get_works() {
        let runnables = test_runnables();
        let mut runs = RunList::new();
        runs.set_runnables(runnables);
        let got_arc = runs.get(1);
        let got = got_arc.lock().unwrap();
        assert_eq!(got.id(), 1)
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

        match runs.next() {
            None => assert!(false),
            Some(arc) => {
                let next = arc.lock().unwrap();
                assert_eq!(next.id(), 0);
            }
        }
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

        match runs.next() {
            None => assert!(false),
            Some(arc) => {
                let next = arc.lock().unwrap();
                assert_eq!(next.id(), 0);
            }
        }
    }
}
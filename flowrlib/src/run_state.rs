use std::sync::{Arc, Mutex};
use std::collections::HashSet;
use process::Process;

pub struct RunState {
    processes: Vec<Arc<Mutex<Process>>>,
    can_run: HashSet<usize>,
    // can_run: HashSet<process_id>
    blocking: Vec<(usize, usize)>,
    // locking: Vec<(blocking_id, blocked_id)>
    will_run: Vec<usize>,
    // will_run: Vec<process_id>
    dispatches: usize,
}

impl RunState {
    pub fn new(processes: Vec<Arc<Mutex<Process>>>) -> Self {
        RunState {
            processes,
            can_run: HashSet::<usize>::new(),
            blocking: Vec::<(usize, usize)>::new(),
            will_run: Vec::<usize>::new(),
            #[cfg(feature = "debugger")]
            dispatches: 0,
        }
    }

    /*
        Reset all values back to inital ones to enable debugging from scracth
    */
    pub fn reset(&mut self) {
        for process_arc in &self.processes {
            let mut process = process_arc.lock().unwrap();
            process.reset()
        };
        self.can_run.clear();
        self.blocking.clear();
        self.will_run.clear();
        if cfg!(feature = "debugger") {
            self.dispatches = 0;
        }
    }

    /*
        The ìnit' function is responsible for initializing all processs.
        The `init` method on each process is called, which returns a boolean to indicate that it's
        inputs are fulfilled - and this information is added to the RunList to control the readyness of
        the Process to be executed.

        Once all processs have been initialized, the list of processs is stored in the RunList
    */
    pub fn init(&mut self) -> usize {
        let mut can_run_list = Vec::<usize>::new();

        for process_arc in &self.processes {
            let mut process = process_arc.lock().unwrap();
            debug!("\tInitializing process #{} '{}'", process.id(), process.name());
            if process.init() {
                can_run_list.push(process.id());
            }
        }

        for id in can_run_list {
            self.can_run(id);
        }

        self.processes.len()
    }

    #[cfg(any(feature = "logging", feature = "debugger"))]
    pub fn print(&self) {
        println!("RunState:");
        println!("   Processes: {}", self.processes.len());
        println!("  Dispatches: {}", self.dispatches);
        println!("     Can Run: {:?}", self.can_run);
        println!("    Blocking: {:?}", self.blocking);
        println!("    Will Run: {:?}", self.will_run);
    }

    #[cfg(any(feature = "metrics", feature = "debugger"))]
    pub fn increment_dispatches(&mut self) {
        self.dispatches += 1;
    }

    pub fn get(&self, id: usize) -> Arc<Mutex<Process>> {
        self.processes[id].clone()
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
            debug!("\t\t\tProcess #{} not blocked on output, so added to 'Will Run' list", id);
            self.will_run.push(id);
        }
    }

    pub fn dispatches(&self) -> usize {
        self.dispatches
    }

    pub fn num_processes(&self) -> usize {
        self.processes.len()
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

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use process::Process;
    use super::RunState;

    fn test_processes<'a>() -> Vec<Arc<Mutex<Process>>> {
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
        let mut state = RunState::new(test_processes());

        // Indicate that 0 is blocked by 1
        state.blocked_by(1, 0);
        assert!(state.is_blocked(0));
    }


    #[test]
    fn get_works() {
        let state = RunState::new(test_processes());
        let got_arc = state.get(1);
        let got = got_arc.lock().unwrap();
        assert_eq!(got.id(), 1)
    }

    #[test]
    fn no_next_if_none_ready() {
        let mut state = RunState::new(test_processes());

        assert!(state.next().is_none());
    }

    #[test]
    fn inputs_ready_makes_ready() {
        let mut state = RunState::new(test_processes());

        // Indicate that 0 has all it's inputs read
        state.can_run(0);

        assert_eq!(state.next().unwrap(), 0);
    }

    #[test]
    fn blocked_is_not_ready() {
        let mut state = RunState::new(test_processes());

        // Indicate that 0 is blocked by 1
        state.blocked_by(1, 0);

        // Indicate that 0 has all it's inputs read
        state.can_run(0);

        match state.next() {
            None => assert!(true),
            Some(_) => assert!(false)
        }
    }

    #[test]
    fn unblocking_makes_ready() {
        let mut state = RunState::new(test_processes());

        // Indicate that 0 is blocked by 1
        state.blocked_by(1, 0);

        // Indicate that 0 has all it's inputs read
        state.can_run(0);

        assert_eq!(state.next(), None);

        // now unblock 0 by 1
        state.unblock_senders_to(1);

        // Now process with id 0 should be ready and served up by next
        assert_eq!(state.next(), Some(0));
    }

    #[test]
    fn unblocking_doubly_blocked_process_not_ready() {
        let mut state = RunState::new(test_processes());

        // Indicate that 0 is blocked by 1 and 2
        state.blocked_by(1, 0);
        state.blocked_by(2, 0);

        // Indicate that 0 has all it's inputs read
        state.can_run(0);

        assert_eq!(state.next(), None);

        // now unblock 0 by 1
        state.unblock_senders_to(1);

        // Now process with id 0 should still not be ready as still blocked on 2
        assert_eq!(state.next(), None);
    }
}
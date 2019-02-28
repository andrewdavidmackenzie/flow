use std::sync::{Arc, Mutex};
use std::collections::HashSet;
use process::Process;

#[derive(Debug)]
enum State {
    Ready,
    // ready to run
    Blocked,
    // cannot run as output is blocked by another process
    Waiting,
    // waiting for inputs to arrive
    Running,     //is being run somewhere
}

pub struct RunState {
    processes: Vec<Arc<Mutex<Process>>>,
    blocked: HashSet<usize>,
    // blocked: HashSet<process_id>
    blocking: Vec<(usize, usize)>,
    // blocking: Vec<(blocking_id, blocked_id)>
    will_run: Vec<usize>,
    // will_run: Vec<process_id>
    running: HashSet<usize>,
    // dispatched: HashSet<process_id>
    dispatches: usize,
}

impl RunState {
    pub fn new(processes: Vec<Arc<Mutex<Process>>>) -> Self {
        RunState {
            processes,
            blocked: HashSet::<usize>::new(),
            blocking: Vec::<(usize, usize)>::new(),
            will_run: Vec::<usize>::new(),
            running: HashSet::<usize>::new(),
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
        self.blocked.clear();
        self.blocking.clear();
        self.will_run.clear();
        self.running.clear();
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
        let mut inputs_ready_list = Vec::<usize>::new();

        for process_arc in &self.processes {
            let mut process = process_arc.lock().unwrap();
            debug!("\tInitializing process #{} '{}'", process.id(), process.name());
            if process.init() {
                inputs_ready_list.push(process.id());
            }
        }

        for id in inputs_ready_list {
            self.inputs_ready(id);
        }

        self.processes.len()
    }

    fn get_state(&self, process_id: usize) -> State {
        if self.will_run.contains(&process_id) {
            State::Ready
        } else {
            if self.blocked.contains(&process_id) {
                State::Blocked
            } else if self.running.contains(&process_id) {
                State::Running
            } else {
                State::Waiting
            }
        }
    }

    #[cfg(feature = "debugger")]
    pub fn get_blocked(&self) -> &HashSet<usize> {
        &self.blocked
    }

    #[cfg(feature = "debugger")]
    pub fn display_state(&self, process_id: usize) -> String {
        let mut output = format!("\tState: {:?}\n", self.get_state(process_id));

        for (blocking, blocked) in &self.blocking {
            if *blocked == process_id {
                output.push_str(&format!("\t\tBlocked #{} --> Blocked by #{}\n", blocked, blocking));
            } else if *blocking == process_id {
                output.push_str(&format!("\t\tBlocking #{} <-- Blocked #{}\n", blocking, blocked));
            }
        }

        output
    }

    #[cfg(any(feature = "logging", feature = "debugger"))]
    pub fn print(&self) {
        println!("RunState:");
        println!("   Processes: {}", self.processes.len());
        println!("  Dispatches: {}", self.dispatches);
        println!("     Blocked: {:?}", self.blocked);
        println!("    Blocking: {:?}", self.blocking);
        println!("    Will Run: {:?}", self.will_run);
        println!("     Running: {:?}", self.running);
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

        // Take the process_id at the head of the will_run list
        let dispatched_id = self.will_run.remove(0);
        self.running.insert(dispatched_id);
        Some(dispatched_id)
    }

    pub fn done(&mut self, id: usize) {
        self.running.remove(&id);
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

    #[cfg(feature = "debugger")]
    pub fn get_output_blockers(&self, id: usize) -> Vec<usize> {
        let mut blockers = vec!();

        for &(blocking_id, blocked_id) in &self.blocking {
            if blocked_id == id {
                blockers.push(blocking_id);
            }
        }

        blockers
    }

    /*
        An input blocker is another process that is the only process connected to an empty input
        of target process, and which is not ready to run, hence target process cannot run.
    */
    #[cfg(feature = "debugger")]
    pub fn get_input_blockers(&self, target_id: usize) -> Vec<usize> {
        let mut input_blockers = vec!();
        let target_process_arc = self.get(target_id);
        let mut target_process_lock = target_process_arc.try_lock();

        if let Ok(ref mut target_process) = target_process_lock {
            // for each empty input of the target process
            for (target_io, input) in target_process.get_inputs().iter().enumerate() {
                if input.is_empty() {
                    let mut senders = Vec::<usize>::new();

                    // go through all processes to see if sends to the target process on input
                    for sender_process_arc in &self.processes {
                        let mut sender_process_lock = sender_process_arc.try_lock();
                        if let Ok(ref mut sender_process) = sender_process_lock {
                            // if the sender process is not ready to run
                            if !self.will_run.contains(&sender_process.id()) {

                                // for each output route of sending process, see if it is sending to the target process and input
                                for &(ref _output_route, destination_id, io_number) in sender_process.output_destinations() {
                                    if (destination_id == target_id) && (io_number == target_io) {
                                        senders.push(sender_process.id());
                                    }
                                }
                            }
                        }
                    }

                    // If unique sender to this Input, then target process is blocked waiting for that value
                    if senders.len() == 1 {
                        input_blockers.extend(senders);
                    }
                }
            }
        }

        input_blockers
    }

    /*
        Save the fact that a particular Process's inputs are now satisfied and so it maybe ready
        to run (if not blocked sending on it's output)
    */
    pub fn inputs_ready(&mut self, id: usize) {
        if self.is_blocked(id) {
            debug!("\t\t\tProcess #{} inputs are ready, but blocked on output", id);
            self.blocked.insert(id);
        } else {
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

            // see if the ones unblocked should be made ready. Note, they could be blocked on other
            // processes apart from the the one that just unblocked it.
            for unblocked in unblocked_list {
                if self.blocked.contains(&unblocked) && !self.is_blocked(unblocked) {
                    debug!("\t\t\tProcess #{} has inputs ready, so removed from 'blocked' and added to 'will_run'", unblocked);
                    self.blocked.remove(&unblocked);
                    self.will_run.push(unblocked);
                }
            }
        }
    }

    // Save the fact that the process 'blocked_id' is blocked on it's output by 'blocking_id'
    pub fn set_blocked_by(&mut self, blocking_id: usize, blocked_id: usize) {
        // avoid deadlocks by a process blocking itself
        if blocked_id != blocking_id {
            debug!("\t\t\tProcess #{} <-- Process #{} blocked", &blocking_id, &blocked_id);
            self.blocking.push((blocking_id, blocked_id));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use process::Process;
    use super::RunState;

    fn test_processes<'a>() -> Vec<Arc<Mutex<Process>>> {
        let p0 = Arc::new(Mutex::new(
            Process::new("p0".to_string(), // name
                         "/context/p0".to_string(),
                         false,// static value
                         "/test".to_string(),
                         vec!(), // input depths array
                         0,    // id
                         None,
                         vec!(("".to_string(), 1, 0), ("".to_string(), 1, 0)), // destinations
            )));    // implementation
        let p1 = Arc::new(Mutex::new(Process::new("p1".to_string(),
                                                  "/context/p1".to_string(),
                                                  false,// static value
                                                  "/test".to_string(),
                                                  vec!(1), // input depths array
                                                  1,    // id
                                                  None,
                                                  vec!(),
        )));
        let p2 = Arc::new(Mutex::new(Process::new("p2".to_string(),
                                                  "/context/p2".to_string(),
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
        state.set_blocked_by(1, 0);
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
        state.inputs_ready(0);

        assert_eq!(state.next().unwrap(), 0);
    }

    #[test]
    fn blocked_is_not_ready() {
        let mut state = RunState::new(test_processes());

// Indicate that 0 is blocked by 1
        state.set_blocked_by(1, 0);

// Indicate that 0 has all it's inputs read
        state.inputs_ready(0);

        match state.next() {
            None => assert!(true),
            Some(_) => assert!(false)
        }
    }

    #[test]
    fn unblocking_makes_ready() {
        let mut state = RunState::new(test_processes());

// Indicate that 0 is blocked by 1
        state.set_blocked_by(1, 0);

// Indicate that 0 has all it's inputs read
        state.inputs_ready(0);

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
        state.set_blocked_by(1, 0);
        state.set_blocked_by(2, 0);

// Indicate that 0 has all it's inputs read
        state.inputs_ready(0);

        assert_eq!(state.next(), None);

// now unblock 0 by 1
        state.unblock_senders_to(1);

// Now process with id 0 should still not be ready as still blocked on 2
        assert_eq!(state.next(), None);
    }
}
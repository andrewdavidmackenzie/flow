use std::sync::{Arc, Mutex};
use std::collections::HashSet;
use function::Function;
use std::fmt;

#[derive(Debug, PartialEq)]
pub enum State {
    Ready,
    // ready to run
    Blocked,
    // cannot run as output is blocked by another function
    Waiting,
    // waiting for inputs to arrive
    Running,     //is being run somewhere
}

/// The Semantics of a Flow's RunState
/// The semantics of the state of each function in a flow and the flow over are described here
/// and the tests of the struct attempt to reproduce and confirm as many of them as is possible
///
/// Initialization
/// ==============
/// Upon initialization all function's inputs are initialized by calling their init_inputs() function.
/// This may initialize one or more inputs with values.
/// This may cause all inputs to be full and hence the Function maybe able to run (pending blocks on other functions).
///
/// States
/// ======
/// Init    - Prior to the initialization process Functions will be in the init state
/// Ready   - Function will be in Ready state when all of it's inputs are full and there are no inputs
///           it sends to that are full (unless that input is it's own)
/// Blocked - Function is in Blocked state when there is at least one input it sends to that is full
///           (unless that input is it's own, as then it will be emptied when the function runs)
/// Waiting - Function is in Blocked state when at least one of it's inputs is not full
/// Running - Function is in Running state when it has been picked from the Ready list for execution
///           using the next() funcion
///
/// State Transitions
/// =================
///
/// From    To State  Event causing transition and additional conditions          Test
/// ----    --------  --------------------------------------------------          ----
/// Init    Ready     Init: No inputs and no destination input full               to_ready_1_on_init
///                   Init: All inputs initialized and no destination input full  to_ready_2_on_init
///                   Init: All inputs initialized and no destinations            to_ready_3_on_init
/// Init    Blocked   Init: Some destination input is full                        to_blocked_on_init
/// Init    Waiting   Init: At least one input is not full                        to_waiting_on_init
///
/// Ready   Running   Next: called to fetch the function_id for execution         ready_to_running_on_next
///
/// Blocked Ready     Done: function(s) blocking some output done                 blocked_to_ready_on_done
///
/// Waiting Ready     Input: a last empty input on a function is filled           waiting_to_ready_on_input
/// Waiting Blocked
///
/// Running Ready     Done: it's inputs are all full, so it can run again         running_to_ready_on_done
/// Running Waiting   Done: it has one input or more empty, to it can't run       running_to_waiting_on_done
/// Running Blocked   Done: at least one destination input is full, so can't run  running_to_blocked_on_done
///
pub struct RunState {
    functions: Vec<Arc<Mutex<Function>>>,
    blocked: HashSet<usize>,
    // blocked: HashSet<function_id>
    blocks: Vec<(usize, usize)>,
    // blocking: Vec<(blocking_id, blocked_id)>
    ready: Vec<usize>,
    // ready: Vec<function_id>
    running: HashSet<usize>,
    // running: HashSet<function_id>
    jobs: usize,
    // number of jobs created to date
    max_jobs: usize,
    // limit on the number of jobs to allow to run in parallel
}

impl RunState {
    pub fn new(functions: Vec<Arc<Mutex<Function>>>, max_jobs: usize) -> Self {
        RunState {
            functions,
            blocked: HashSet::<usize>::new(),
            blocks: Vec::<(usize, usize)>::new(),
            ready: Vec::<usize>::new(),
            running: HashSet::<usize>::new(),
            #[cfg(feature = "debugger")]
            jobs: 0,
            max_jobs,
        }
    }

    /*
        Reset all values back to inital ones to enable debugging from scracth
    */
    pub fn reset(&mut self) {
        for function_arc in &self.functions {
            let mut function = function_arc.lock().unwrap();
            function.reset()
        };
        self.blocked.clear();
        self.blocks.clear();
        self.ready.clear();
        self.running.clear();
        if cfg!(feature = "debugger") {
            self.jobs = 0;
        }
    }

    /*
        The ìnit' function is responsible for initializing all functions.
        The `init` method on each function is called, which returns a boolean to indicate that it's
        inputs are fulfilled - and this information is added to the RunList to control the readyness of
        the Process to be executed.
    */
    pub fn init(&mut self) {
        let mut inputs_ready_list = Vec::<usize>::new();

        for function_arc in &self.functions {
            let mut function = function_arc.lock().unwrap();
            debug!("\tInitializing Function #{} '{}'", function.id(), function.name());
            function.init_inputs(true);
            if function.inputs_full() {
                inputs_ready_list.push(function.id());
            }
            drop(function);
        }

        // Due to initialization of some inputs other functions attempting to send to it should block
        self.create_init_blocks();

        // Put all functions that have their inputs ready on the appropriate list
        for id in inputs_ready_list {
            self.inputs_now_full(id);
        }
    }

    /*
        Scan thru all functions and output routes for each, if the destination input is already
        full due to the init process, then create a block for the sender
    */
    fn create_init_blocks(&mut self) {
        let mut blocks = Vec::<(usize, usize)>::new();

        for source_function_arc in &self.functions {
            let source_id;
            let destinations;
            {
                let source_function = source_function_arc.lock().unwrap();
                source_id = source_function.id();
                destinations = source_function.output_destinations().clone();
                drop(&source_function);
            }
            for (_, destination_id, io_number) in destinations {
                if destination_id != source_id { // don't block yourself!
                    let destination_function_arc = self.get(destination_id);
                    let destination_function = destination_function_arc.try_lock().unwrap();
                    if destination_function.input_full(io_number) {
                        blocks.push((destination_id, source_id));
                    }
                }
            }
        }

        self.blocks = blocks;
    }

    pub fn get_state(&self, function_id: usize) -> State {
        if self.ready.contains(&function_id) {
            State::Ready
        } else {
            if self.blocked.contains(&function_id) {
                State::Blocked
            } else if self.running.contains(&function_id) {
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
    pub fn display_state(&self, function_id: usize) -> String {
        let mut output = format!("\tState: {:?}\n", self.get_state(function_id));

        for (blocking, blocked) in &self.blocks {
            if *blocked == function_id {
                output.push_str(&format!("\t\tBlocked #{} --> Blocked by #{}\n", blocked, blocking));
            } else if *blocking == function_id {
                output.push_str(&format!("\t\tBlocking #{} <-- Blocked #{}\n", blocking, blocked));
            }
        }

        output
    }

    #[cfg(any(feature = "metrics", feature = "debugger"))]
    pub fn increment_jobs(&mut self) {
        self.jobs += 1;
    }

    pub fn get(&self, id: usize) -> Arc<Mutex<Function>> {
        self.functions[id].clone()
    }

    /*
        Return the id of the next function ready to be run, if there is one and there are not
        too many jobs already running
    */
    pub fn next(&mut self) -> Option<usize> {
        if self.ready.is_empty() || self.running.len() >= self.max_jobs {
            return None;
        }

        // Take the function_id at the head of the ready list
        let function_id = self.ready.remove(0);
        self.running.insert(function_id);
        Some(function_id)
    }

    pub fn done(&mut self, id: usize) {
        self.running.remove(&id);
    }

    // TODO use the blocked_id as a key to a HashSet?
    // See if there is any tuple in the vector where the second (blocked_id) is the one we're after
    pub fn is_blocked(&self, id: usize) -> bool {
        for &(_blocking_id, blocked_id) in &self.blocks {
            if blocked_id == id {
                return true;
            }
        }
        false
    }

    #[cfg(feature = "debugger")]
    pub fn get_output_blockers(&self, id: usize) -> Vec<usize> {
        let mut blockers = vec!();

        for &(blocking_id, blocked_id) in &self.blocks {
            if blocked_id == id {
                blockers.push(blocking_id);
            }
        }

        blockers
    }

    pub fn number_jobs_running(&self) -> usize {
        self.running.len()
    }

    pub fn number_jobs_ready(&self) -> usize {
        self.ready.len()
    }

    /*
        An input blocker is another function that is the only function connected to an empty input
        of target function, and which is not ready to run, hence target function cannot run.
    */
    #[cfg(feature = "debugger")]
    pub fn get_input_blockers(&self, target_id: usize) -> Vec<usize> {
        let mut input_blockers = vec!();
        let target_function_arc = self.get(target_id);
        let mut target_function_lock = target_function_arc.try_lock();

        if let Ok(ref mut target_functions) = target_function_lock {
            // for each empty input of the target function
            for (target_io, input) in target_functions.inputs().iter().enumerate() {
                if input.is_empty() {
                    let mut senders = Vec::<usize>::new();

                    // go through all functions to see if sends to the target function on input
                    for sender_function_arc in &self.functions {
                        let mut sender_function_lock = sender_function_arc.try_lock();
                        if let Ok(ref mut sender_function) = sender_function_lock {
                            // if the sender function is not ready to run
                            if !self.ready.contains(&sender_function.id()) {

                                // for each output route of sending function, see if it is sending to the target function and input
                                for (ref _output_route, destination_id, io_number) in sender_function.output_destinations() {
                                    if (*destination_id == target_id) && (*io_number == target_io) {
                                        senders.push(sender_function.id());
                                    }
                                }
                            }
                        }
                    }

                    // If unique sender to this Input, then target function is blocked waiting for that value
                    if senders.len() == 1 {
                        input_blockers.extend(senders);
                    }
                }
            }
        }

        input_blockers
    }

    /*
        Save the fact that a particular Function's inputs are now full and so it maybe ready
        to run (if not blocked sending on it's output)
    */
    pub fn inputs_now_full(&mut self, id: usize) {
        if self.is_blocked(id) {
            debug!("\t\t\tProcess #{} inputs are ready, but blocked on output", id);
            self.blocked.insert(id);
        } else {
            debug!("\t\t\tProcess #{} not blocked on output, so added to 'Will Run' list", id);
            self.ready.push(id);
        }
    }

    pub fn jobs(&self) -> usize {
        self.jobs
    }

    pub fn num_functions(&self) -> usize {
        self.functions.len()
    }

    /*
        unblock all functions that were blocked trying to send to blocker_id by removing all entries
        in the list where the first value (blocking_id) matches the destination_id
        when each is unblocked on output, if it's inputs are satisfied, then it is ready to be run
        again, so put it on the ready queue
    */
    pub fn unblock_senders_to(&mut self, blocker_id: usize) {
        if !self.blocks.is_empty() {
            let mut unblocked_list = vec!();

            for &(blocking_id, blocked_id) in &self.blocks {
                if blocking_id == blocker_id {
                    debug!("\t\tProcess #{} <-- #{} - blocked removed", blocking_id, blocked_id);
                    unblocked_list.push(blocked_id);
                }
            }

            // remove all blocks from the blocking list where the blocker was blocker_id
            self.blocks.retain(|&(blocking_id, _blocked_id)| blocking_id != blocker_id);

            // see if the functions unblocked should no be made ready.
            // Note, they could be blocked on other functions apart from the the one that just unblocked
            for unblocked in unblocked_list {
                if self.blocked.contains(&unblocked) && !self.is_blocked(unblocked) {
                    debug!("\t\t\tProcess #{} has inputs ready, so removed from 'blocked' and added to 'ready'", unblocked);
                    self.blocked.remove(&unblocked);
                    self.ready.push(unblocked);
                }
            }
        }
    }

    /*
        Create a 'block" indicating that function 'blocked_id' cannot run as it has an output
        destination to an input on function 'blocking_id' that is already full.

        Avoid deadlocks caused by a function blocking itself
    */
    pub fn set_blocked_by(&mut self, blocking_id: usize, blocked_id: usize) {
        if blocked_id != blocking_id {
            debug!("\t\t\tProcess #{} <-- Process #{} blocked", &blocking_id, &blocked_id);
            self.blocks.push((blocking_id, blocked_id));
        }
    }
}

#[cfg(any(feature = "logging", feature = "debugger"))]
impl fmt::Display for RunState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "RunState:")?;
        write!(f, "   Processes: {}", self.functions.len())?;
        write!(f, "        Jobs: {}", self.jobs)?;
        write!(f, "     Blocked: {:?}", self.blocked)?;
        write!(f, "    Blocking: {:?}", self.blocks)?;
        write!(f, "    Will Run: {:?}", self.ready)?;
        write!(f, "     Running: {:?}", self.running)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use function::Function;
    use super::RunState;
    use super::State;
    use input::InputInitializer::OneTime;
    use input::OneTimeInputInitializer;

    /********************************* State Transition Tests *********************************/
    #[test]
    fn to_ready_1_on_init() {
        let f_a = Arc::new(Mutex::new(
            Function::new("fA".to_string(), // name
                          "/context/fA".to_string(),
                          "/test".to_string(),
                          false,
                          vec!(),
                          0,
                          vec!(("".to_string(), 1, 0)), // outputs to f_b:0
            )));
        let f_b = Arc::new(Mutex::new(
            Function::new("fB".to_string(), // name
                          "/context/fB".to_string(),
                          "/test".to_string(),
                          false,
                          vec!((1, None)),
                          1,
                          vec!(),
            )));
        let functions = vec!(f_a, f_b);
        let mut state = RunState::new(functions, 1);
        state.init();
        assert_eq!(State::Ready, state.get_state(0), "f_a should be Ready");
    }

    #[test]
    fn to_ready_2_on_init() {
        let f_a = Arc::new(Mutex::new(
            Function::new("fA".to_string(), // name
                          "/context/fA".to_string(),
                          "/test".to_string(),
                          false,
                          vec!((1, Some(OneTime(OneTimeInputInitializer { once: json!(1) })))),
                          0,
                          vec!(("".to_string(), 1, 0)), // outputs to fB:0
            )));
        let f_b = Arc::new(Mutex::new(
            Function::new("fB".to_string(), // name
                          "/context/fB".to_string(),
                          "/test".to_string(),
                          false,
                          vec!((1, None)),
                          1,
                          vec!(),
            )));
        let functions = vec!(f_a, f_b);
        let mut state = RunState::new(functions, 1);
        state.init();
        assert_eq!(State::Ready, state.get_state(0), "f_a should be Ready");
    }

    #[test]
    fn to_ready_3_on_init() {
        let f_a = Arc::new(Mutex::new(
            Function::new("fA".to_string(), // name
                          "/context/fA".to_string(),
                          "/test".to_string(),
                          false,
                          vec!((1, Some(OneTime(OneTimeInputInitializer { once: json!(1) })))),
                          0,
                          vec!(),
            )));
        let functions = vec!(f_a);
        let mut state = RunState::new(functions, 1);
        state.init();
        assert_eq!(State::Ready, state.get_state(0), "f_a should be Ready");
    }

    /*
        FunctionA -> FunctionB
        But FunctionB has an initializer on that same input and FunctionB is initialized before
        FunctionA, so the input should be full and when FunctionA initializes it should go to blocked
        status
    */
    #[test]
    fn to_blocked_on_init() {
        let f_a = Arc::new(Mutex::new(
            Function::new("fA".to_string(), // name
                          "/context/fA".to_string(),
                          "/test".to_string(),
                          false,
                          vec!((1, Some(OneTime(OneTimeInputInitializer { once: json!(1) })))),
                          0,
                          vec!(("".to_string(), 1, 0)), // outputs to fB:0
            )));
        let f_b = Arc::new(Mutex::new(
            Function::new("fB".to_string(), // name
                          "/context/fB".to_string(),
                          "/test".to_string(),
                          false,
                          vec!((1, Some(OneTime(OneTimeInputInitializer { once: json!(1) })))),
                          1,
                          vec!(),
            )));
        let functions = vec!(f_b, f_a);
        let mut state = RunState::new(functions, 1);
        state.init();
        assert_eq!(State::Ready, state.get_state(1), "f_b should be Ready");
        assert_eq!(State::Blocked, state.get_state(0), "f_a should be in Blocked state, by fB");
    }

    #[test]
    fn to_waiting_on_init() {
        let f_a = Arc::new(Mutex::new(
            Function::new("fA".to_string(), // name
                          "/context/fA".to_string(),
                          "/test".to_string(),
                          false,
                          vec!((1, None)),
                          0,
                          vec!(),
            )));
        let functions = vec!(f_a);
        let mut state = RunState::new(functions, 1);
        state.init();
        assert_eq!(State::Waiting, state.get_state(0), "f_a should be Waiting");
    }

    #[test]
    fn ready_to_running_on_next() {
        let f_a = Arc::new(Mutex::new(
            Function::new("fA".to_string(), // name
                          "/context/fA".to_string(),
                          "/test".to_string(),
                          false,
                          vec!((1, Some(OneTime(OneTimeInputInitializer { once: json!(1) })))),
                          0,
                          vec!(),
            )));
        let functions = vec!(f_a);
        let mut state = RunState::new(functions, 1);
        state.init();
        assert_eq!(State::Ready, state.get_state(0), "f_a should be Ready");
        assert_eq!(0, state.next().unwrap(), "next() should return function_id = 0");
        assert_eq!(State::Running, state.get_state(0), "f_a should be Running");
    }

    #[test]
    fn unready_not_to_running_on_next() {
        let f_a = Arc::new(Mutex::new(
            Function::new("fA".to_string(), // name
                          "/context/fA".to_string(),
                          "/test".to_string(),
                          false,
                          vec!((1, None)),
                          0,
                          vec!(),
            )));
        let functions = vec!(f_a);
        let mut state = RunState::new(functions, 1);
        state.init();
        assert_eq!(State::Waiting, state.get_state(0), "f_a should be Waiting");
        assert_eq!(None, state.next(), "next() should return None");
        assert_eq!(State::Waiting, state.get_state(0), "f_a should be Waiting");
    }

    #[test]
    fn blocked_to_ready_on_done() {
        let f_a = Arc::new(Mutex::new(
            Function::new("fA".to_string(), // name
                          "/context/fA".to_string(),
                          "/test".to_string(),
                          false,
                          vec!((1, Some(OneTime(OneTimeInputInitializer { once: json!(1) })))),
                          0,
                          vec!(("".to_string(), 1, 0)), // outputs to fB:0
            )));
        let f_b = Arc::new(Mutex::new(
            Function::new("fB".to_string(), // name
                          "/context/fB".to_string(),
                          "/test".to_string(),
                          false,
                          vec!((1, Some(OneTime(OneTimeInputInitializer { once: json!(1) })))),
                          1,
                          vec!(),
            )));
        let functions = vec!(f_b, f_a); // NOTE the order!
        let mut state = RunState::new(functions, 1);
        state.init();
        assert_eq!(State::Ready, state.get_state(1), "f_b should be Ready");
        assert_eq!(State::Blocked, state.get_state(0), "f_a should be in Blocked state, by fB");
        assert_eq!(1, state.next().unwrap(), "next() should return function_id=1 (f_b) for running");
        state.unblock_senders_to(1);
        state.done(1); // Mark function_id=1 (f_b) as having ran
        assert_eq!(State::Ready, state.get_state(0), "f_a should be Ready");
    }

    #[test]
    fn running_to_ready_on_done() {
        let f_a = Arc::new(Mutex::new(
            Function::new("fA".to_string(), // name
                          "/context/fA".to_string(),
                          "/test".to_string(),
                          false,
                          vec!((1, Some(OneTime(OneTimeInputInitializer { once: json!(1) })))),
                          0,
                          vec!(),
            )));
        let functions = vec!(f_a);
        let mut state = RunState::new(functions, 1);
        state.init();
        assert_eq!(State::Ready, state.get_state(0), "f_a should be Ready");
        assert_eq!(0, state.next().unwrap(), "next() should return function_id = 0");
        assert_eq!(State::Running, state.get_state(0), "f_a should be Running");

        // This is done by coordinator in update_states()...
        let source_arc = state.get(0);
        let mut f_a = source_arc.lock().unwrap();
        f_a.init_inputs(false);
        if f_a.inputs_full() {
            state.inputs_now_full(0);
        }

        // Then Coordinator marks it as "done"
        state.done(0); // Mark function_id=0 (f_a) as having ran
        assert_eq!(State::Ready, state.get_state(0), "f_a should be Ready again");
    }

    // Done: it has one input or more empty, to it can't run
    #[test]
    fn running_to_waiting_on_done() {
        let f_a = Arc::new(Mutex::new(
            Function::new("fA".to_string(), // name
                          "/context/fA".to_string(),
                          "/test".to_string(),
                          false,
                          vec!((1, Some(OneTime(OneTimeInputInitializer { once: json!(1) })))),
                          0,
                          vec!(),
            )));
        let functions = vec!(f_a);
        let mut state = RunState::new(functions, 1);
        state.init();
        assert_eq!(State::Ready, state.get_state(0), "f_a should be Ready");
        assert_eq!(0, state.next().unwrap(), "next() should return function_id = 0");
        assert_eq!(State::Running, state.get_state(0), "f_a should be Running");

        // Then Coordinator marks it as "done"
        state.done(0); // Mark function_id=0 (f_a) as having ran
        assert_eq!(State::Waiting, state.get_state(0), "f_a should be Waiting again");
    }

    // Done: at least one destination input is full, so can't run  running_to_blocked_on_done
    #[test]
    fn running_to_blocked_on_done() {
        let f_a = Arc::new(Mutex::new(
            Function::new("fA".to_string(), // name
                          "/context/fA".to_string(),
                          "/test".to_string(),
                          false,
                          vec!((1, Some(OneTime(OneTimeInputInitializer { once: json!(1) })))),
                          0,
                          vec!(("".to_string(), 1, 0)), // outputs to fB:0
            )));
        let f_b = Arc::new(Mutex::new(
            Function::new("fB".to_string(), // name
                          "/context/fB".to_string(),
                          "/test".to_string(),
                          false,
                          vec!((1, None)),
                          1,
                          vec!(),
            )));
        let functions = vec!(f_a, f_b);
        let mut state = RunState::new(functions, 1);
        state.init();

        assert_eq!(State::Ready, state.get_state(0), "f_a should be Ready");

        assert_eq!(0, state.next().unwrap(), "next() should return function_id=0 (f_a) for running");
        assert_eq!(State::Running, state.get_state(0), "f_a should be Running");

        // f_a runs and sends to f_b
        state.inputs_now_full(1);
        state.set_blocked_by(1, 0);

        // While running, someone else sends to f_a's input - having to call this is not idea...
        // done() should just figure it all out at the end?
        state.inputs_now_full(0);

        // Mark function_id=0 (f_a) as having ran
        state.done(0);

        // f_a should transition to Blocked on f_b
        assert_eq!(State::Blocked, state.get_state(0), "f_a should be Blocked");
    }

    #[test]
    fn waiting_to_ready_on_input() {
        let f_a = Arc::new(Mutex::new(
            Function::new("fA".to_string(), // name
                          "/context/fA".to_string(),
                          "/test".to_string(),
                          false,
                          vec!((1, None)),
                          0,
                          vec!(),
            )));
        let functions = vec!(f_a);
        let mut state = RunState::new(functions, 1);
        state.init();
        assert_eq!(State::Waiting, state.get_state(0), "f_a should be Waiting");

        // This is done by coordinator in update_states()...
        let function_arc = state.get(0);
        let mut f_a = function_arc.lock().unwrap();
        f_a.write_input(0, json!(1));
        if f_a.inputs_full() {
            state.inputs_now_full(0);
        }

        // Then Coordinator marks it as "done"
        state.done(0); // Mark function_id=0 (f_a) as having ran
        assert_eq!(State::Ready, state.get_state(0), "f_a should be Ready");
    }

    /****************************** Miscelaneous tests **************************/

    fn test_functions<'a>() -> Vec<Arc<Mutex<Function>>> {
        let p0 = Arc::new(Mutex::new(
            Function::new("p0".to_string(), // name
                          "/context/p0".to_string(),
                          "/test".to_string(),
                          false, vec!(), // input depths array
                          0,    // id
                          vec!(("".to_string(), 1, 0), ("".to_string(), 1, 0)), // destinations
            )));    // implementation
        let p1 = Arc::new(Mutex::new(Function::new("p1".to_string(),
                                                   "/context/p1".to_string(),
                                                   "/test".to_string(),
                                                   false, vec!((1, None)), // inputs array
                                                   1,    // id
                                                   vec!(),
        )));
        let p2 = Arc::new(Mutex::new(Function::new("p2".to_string(),
                                                   "/context/p2".to_string(),
                                                   "/test".to_string(),
                                                   false, vec!((1, None)), // inputs array
                                                   2,    // id
                                                   vec!(),
        )));
        vec!(p0, p1, p2)
    }


    #[test]
    fn blocked_works() {
        let mut state = RunState::new(test_functions(), 1);

        // Indicate that 0 is blocked by 1
        state.set_blocked_by(1, 0);
        assert!(state.is_blocked(0));
    }

    #[test]
    fn get_works() {
        let state = RunState::new(test_functions(), 1);
        let got_arc = state.get(1);
        let got = got_arc.lock().unwrap();
        assert_eq!(got.id(), 1)
    }

    #[test]
    fn no_next_if_none_ready() {
        let mut state = RunState::new(test_functions(), 1);

        assert!(state.next().is_none());
    }

    #[test]
    fn next_works() {
        let mut state = RunState::new(test_functions(), 1);

        // Put 0 on the blocked/ready
        state.inputs_now_full(0);

        assert_eq!(state.next().unwrap(), 0);
    }

    #[test]
    fn inputs_ready_makes_ready() {
        let mut state = RunState::new(test_functions(), 1);

        // Put 0 on the blocked/ready list depending on blocked status
        state.inputs_now_full(0);

        assert_eq!(state.next().unwrap(), 0);
    }

    #[test]
    fn blocked_is_not_ready() {
        let mut state = RunState::new(test_functions(), 1);

        // Indicate that 0 is blocked by 1
        state.set_blocked_by(1, 0);

        // Put 0 on the blocked/ready list depending on blocked status
        state.inputs_now_full(0);

        match state.next() {
            None => assert!(true),
            Some(_) => assert!(false)
        }
    }

    #[test]
    fn unblocking_makes_ready() {
        let mut state = RunState::new(test_functions(), 1);

        // Indicate that 0 is blocked by 1
        state.set_blocked_by(1, 0);

        // Put 0 on the blocked/ready list depending on blocked status
        state.inputs_now_full(0);

        assert_eq!(state.next(), None);

        // now unblock 0 by 1
        state.unblock_senders_to(1);

        // Now function with id 0 should be ready and served up by next
        assert_eq!(state.next(), Some(0));
    }

    #[test]
    fn unblocking_doubly_blocked_functions_not_ready() {
        let mut state = RunState::new(test_functions(), 1);

        // Indicate that 0 is blocked by 1 and 2
        state.set_blocked_by(1, 0);
        state.set_blocked_by(2, 0);

        // Put 0 on the blocked/ready list depending on blocked status
        state.inputs_now_full(0);

        assert_eq!(state.next(), None);

        // now unblock 0 by 1
        state.unblock_senders_to(1);

        // Now function with id 0 should still not be ready as still blocked on 2
        assert_eq!(state.next(), None);
    }

    #[test]
    fn wont_return_too_many_jobs() {
        let mut state = RunState::new(test_functions(), 1);

        // Put 0 on the blocked/ready
        state.inputs_now_full(0);
        // Put 1 on the blocked/ready
        state.inputs_now_full(1);

        assert_eq!(state.next().unwrap(), 0);
        assert_eq!(state.next(), None);
    }
}
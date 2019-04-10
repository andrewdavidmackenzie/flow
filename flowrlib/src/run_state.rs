use std::sync::{Arc, Mutex};
use std::collections::HashSet;
use function::Function;
use std::fmt;
use implementation::Implementation;
use serde_json::Value;
use metrics::Metrics;
use debugger::Debugger;

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

pub struct Job {
    pub function_id: usize,
    pub implementation: Arc<Implementation>,
    pub input_values: Vec<Vec<Value>>,
    pub destinations: Vec<(String, usize, usize)>,
    pub impure: bool,
}

#[derive(Debug)]
pub struct Output {
    pub function_id: usize,
    pub input_values: Vec<Vec<Value>>,
    pub result: (Option<Value>, bool),
    pub destinations: Vec<(String, usize, usize)>,
    pub error: Option<String>,
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
/// Ready   Running   NextJob: called to fetch the function_id for execution      ready_to_running_on_next
///
/// Blocked Ready     Output: function that was blocking another completes        blocked_to_ready_on_done
///
/// Waiting Ready     Output: last empty input on a function is filled            waiting_to_ready_on_input
/// Waiting Blocked   Output: last empty input on a function is filled, blocked   waiting_to_blocked_on_input
///
/// Running Ready     Output: it's inputs are all full, so it can run again       running_to_ready_on_done
/// Running Waiting   Output: it has one input or more empty, to it can't run     running_to_waiting_on_done
/// Running Blocked   Output: a destination input is full, so can't run           running_to_blocked_on_done
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
        the Fucntion to be executed.

        After init() Functions will either be:
           - Ready:   an entry will be added to the `ready` list with this function's id
           - Blocked: the function has all it's inputs ready and could run but a Function it sends to
                      has an input full already (due to being initialized during the init process)
                      - an entry will be added to the `blocks` list with this function's id as source_id
                      - an entry will be added to the `blocked` list with this function's id
           - Waiting: function has at least one empty input so it cannot run. It will not added to
                      `ready` nor `blocked` lists, so by omission it is in the `Waiting` state.
                      But the `block` will be created so when later it's inputs become full the fact
                      it is blocked will be detected and it can move to the `blocked` state

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

        // Put all functions that have their inputs ready and are not blocked on the `ready` list
        for id in inputs_ready_list {
            self.inputs_now_full(id);
        }
    }

    /*
        Scan thru all functions and output routes for each, if the destination input is already
        full due to the init process, then create a block for the sender and added sender to blocked
        list.
    */
    fn create_init_blocks(&mut self) {
        let mut blocks = Vec::<(usize, usize)>::new();
        let mut blocked = HashSet::<usize>::new();

        debug!("Creating any initial block entries that are needed");

        for source_function_arc in &self.functions {
            let source_id;
            let destinations;
            let source_has_inputs_full;
            {
                let source_function = source_function_arc.lock().unwrap();
                source_id = source_function.id();
                source_has_inputs_full = source_function.inputs_full();
                destinations = source_function.output_destinations().clone();
                drop(&source_function);
            }
            for (_, destination_id, io_number) in destinations {
                if destination_id != source_id { // don't block yourself!
                    let destination_function_arc = self.get(destination_id);
                    let destination_function = destination_function_arc.try_lock().unwrap();
                    if destination_function.input_full(io_number) {
                        debug!("\tAdded block between #{} <-- #{}", destination_id, source_id);
                        blocks.push((destination_id, source_id));
                        // only put source on the blocked list if it already has it's inputs full
                        if source_has_inputs_full {
                            blocked.insert(source_id);
                        }
                    }
                }
            }
        }

        self.blocks = blocks;
        self.blocked = blocked;
    }

    fn get_state(&self, function_id: usize) -> State {
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
        Return the next job ready to be run, if there is one and there are not
        too many jobs already running
    */
    pub fn next_job(&mut self) -> Option<Job> {
        if self.ready.is_empty() || self.running.len() >= self.max_jobs {
            return None;
        }

        // Take the function_id at the head of the ready list
        let function_id = self.ready.remove(0);
        self.running.insert(function_id);

        Some(self.create_job(function_id))
    }

    /*
        Given a function id, prepare a job for execution that contains the input values, the
        implementation and the destination functions the output should be sent to when done
    */
    fn create_job(&mut self, id: usize) -> Job {
        let function_arc = self.get(id);
        let function: &mut Function = &mut *function_arc.lock().unwrap();

        let input_values = function.take_input_values();

        self.unblock_senders_to(id);
        debug!("Preparing Job for Function #{} '{}' with inputs: {:?}", id, function.name(), input_values);

        let implementation = function.get_implementation();

        #[cfg(any(feature = "metrics", feature = "debugger"))]
            self.increment_jobs();

        let destinations = function.output_destinations().clone();

        Job { function_id: id, implementation, input_values, destinations, impure: function.is_impure() }
    }

    /*
        Take an output produced by a function and modify the runlist accordingly
        If other functions were blocked trying to send to this one - we can now unblock them
        as it has consumed it's inputs and they are free to be sent to again.

        Then take the output and send it to all destination IOs on different function it should be
        sent to, marking the source function as blocked because those others must consume the output
        if those other function have all their inputs, then mark them accordingly.
    */
    pub fn process_output(&mut self, metrics: &mut Metrics, output: Output,
                          display_output: bool, debugger: &mut Debugger) {
        match output.error {
            None => {
                let output_value = output.result.0;
                let source_can_run_again = output.result.1;

                debug!("\tCompleted Function #{}", output.function_id);
                if cfg!(feature = "debugger") && display_output {
                    debugger.client.display(&format!("Completed Function #{}\n", output.function_id));
                }

                // did it produce any output value
                if let Some(output_v) = output_value {
                    debug!("\tProcessing output '{}' from Function #{}", output_v, output.function_id);

                    if cfg!(feature = "debugger") && display_output {
                        debugger.client.display(&format!("\tProduced output {}\n", &output_v));
                    }

                    for (ref output_route, destination_id, io_number) in output.destinations {
                        let output_value = output_v.pointer(&output_route).unwrap();
                        debug!("\t\tFunction #{} sent value '{}' via output route '{}' to Function #{} input :{}",
                               output.function_id, output_value, output_route, &destination_id, &io_number);
                        if cfg!(feature = "debugger") && display_output {
                            debugger.client.display(
                                &format!("\t\tSending to {}:{}\n", destination_id, io_number));
                        }

                        #[cfg(feature = "debugger")]
                            debugger.watch_data(self, output.function_id, output_route,
                                                     &output_value, destination_id, io_number);

                        self.send_value(output.function_id, destination_id,
                                         io_number, output_value.clone(), metrics, debugger);
                    }
                }

                // if it wants to run again, and after possibly refreshing any constant inputs, it can
                // (it's inputs are ready) then add back to the Will Run list
                if source_can_run_again {
                    self.refresh_inputs(output.function_id);
                }
            }
            Some(_) => error!("Error in Job execution:\n{:?}", output)
        }

        // remove from the running list
        self.done(output.function_id);
    }

    fn send_value(&mut self, source_id: usize, destination_id: usize, io_number: usize,
                      output_value: Value, metrics: &mut Metrics, debugger: &mut Debugger) {
        let destination_arc = self.get(destination_id);
        let mut destination = destination_arc.lock().unwrap();

        // to another, and it sets the correct state on both.
        destination.write_input(io_number, output_value);

        #[cfg(feature = "metrics")]
            metrics.increment_outputs_sent();

        if destination.input_full(io_number) {
            self.set_blocked_by(destination_id, source_id);
            #[cfg(feature = "debugger")]
                debugger.check_block(self, destination_id, source_id);
        }

        // for the case when a function is sending to itself, delay determining if it should
        // be in the blocked or ready lists until it has sent all it's other outputs
        // as it might be blocked by another function.
        // If not, this will be fixed in the "if source_can_run_again" block below
        if destination.inputs_full() && (source_id != destination_id) {
            self.inputs_now_full(destination_id);
        }
    }

    fn refresh_inputs(&mut self, id: usize) {
        let source_arc = self.get(id);
        let mut source = source_arc.lock().unwrap();

        // refresh any constant inputs it may have
        source.init_inputs(false);

        if source.inputs_full() {
            self.inputs_now_full(id);
        }
    }

    fn done(&mut self, id: usize) {
        self.running.remove(&id);
    }

    // TODO use the blocked_id as a key to a HashSet?
    // See if there is any tuple in the vector where the second (blocked_id) is the one we're after
    fn is_blocked(&self, id: usize) -> bool {
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
    fn inputs_now_full(&mut self, id: usize) {
        if self.is_blocked(id) {
            debug!("\t\t\tFunction #{} inputs are ready, but blocked on output", id);
            self.blocked.insert(id);
        } else {
            debug!("\t\t\tFunction #{} not blocked on output, so added to 'Will Run' list", id);
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
    fn unblock_senders_to(&mut self, blocker_id: usize) {
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
    fn set_blocked_by(&mut self, blocking_id: usize, blocked_id: usize) {
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
    use super::Output;
    use input::InputInitializer::{OneTime, Constant};
    use input::{OneTimeInputInitializer, ConstantInputInitializer};
    use metrics::Metrics;
    use debugger::Debugger;
    use debug_client::DebugClient;
    use std::io;
    use std::io::Write;

    // Helpers
    struct TestDebugClient {}

    impl DebugClient for TestDebugClient {
        fn init(&self) {}

        fn display(&self, output: &str) {
            print!("{}", output);
            io::stdout().flush().unwrap();
        }
        fn read_input(&self, input: &mut String) -> io::Result<usize> {
            io::stdin().read_line(input)
        }
    }

    fn test_debug_client() -> &'static DebugClient {
        &TestDebugClient{}
    }

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

        // Event
        state.init();

        // Test
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

        // Event
        state.init();

        // Test
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

        // Event
        state.init();

        // Test
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

        // Event
        state.init();

        // Test
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

        // Event
        state.init();

        // Test
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

        // Event
        assert_eq!(0, state.next_job().unwrap().function_id, "next_job() should return function_id = 0");

        // Test
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

        // Event
        assert!(state.next_job().is_none(), "next_job() should return None");

        // Test
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
        let mut metrics = Metrics::new(2);
        let mut debugger = Debugger::new(test_debug_client());
        state.init();
        assert_eq!(State::Ready, state.get_state(1), "f_b should be Ready");
        assert_eq!(State::Blocked, state.get_state(0), "f_a should be in Blocked state, by fB");
        assert_eq!(1, state.next_job().unwrap().function_id, "next() should return function_id=1 (f_b) for running");

        // Event
        let output = Output {
            function_id: 1,
            input_values: vec!(vec!(json!(1))),
            result: (Some(json!(1)), true),
            destinations: vec!(("".into(), 1, 0)),
            error: None,

        };
        state.process_output(&mut metrics, output, false, &mut debugger);

        // Test
        assert_eq!(State::Ready, state.get_state(0), "f_a should be Ready");
    }

    #[test]
    fn running_to_ready_on_done() {
        let f_a = Arc::new(Mutex::new(
            Function::new("fA".to_string(), // name
                          "/context/fA".to_string(),
                          "/test".to_string(),
                          false,
                          vec!((1, Some(Constant(ConstantInputInitializer { constant: json!(1) })))),
                          0,
                          vec!(),
            )));
        let functions = vec!(f_a);
        let mut state = RunState::new(functions, 1);
        let mut metrics = Metrics::new(1);
        let mut debugger = Debugger::new(test_debug_client());
        state.init();
        assert_eq!(State::Ready, state.get_state(0), "f_a should be Ready");
        assert_eq!(0, state.next_job().unwrap().function_id, "next() should return function_id = 0");
        assert_eq!(State::Running, state.get_state(0), "f_a should be Running");

        // Event
        let output = Output {
            function_id: 0,
            input_values: vec!(vec!(json!(1))),
            result: (None, true),
            destinations: vec!(),
            error: None,
        };
        state.process_output(&mut metrics, output, false, &mut debugger);

        // Test
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
        assert_eq!(0, state.next_job().unwrap().function_id, "next() should return function_id = 0");
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

        assert_eq!(0, state.next_job().unwrap().function_id, "next() should return function_id=0 (f_a) for running");
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
        state.inputs_now_full(0);

        // Then Coordinator marks it as "done"
        state.done(0); // Mark function_id=0 (f_a) as having ran
        assert_eq!(State::Ready, state.get_state(0), "f_a should be Ready");
    }

    #[test]
    fn waiting_to_blocked_on_input() {
        let f_a = Arc::new(Mutex::new(
            Function::new("fA".to_string(), // name
                          "/context/fA".to_string(),
                          "/test".to_string(),
                          false,
                          vec!((1, None)),
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
        let functions = vec!(f_a, f_b);
        let mut state = RunState::new(functions, 1);
        state.init();

        assert_eq!(State::Ready, state.get_state(1), "f_b should be Ready");
        assert_eq!(State::Waiting, state.get_state(0), "f_a should be in Waiting");

        // This is done by coordinator in update_states()...
        state.inputs_now_full(0);

        // Then Coordinator marks it as "done"
        state.done(0); // Mark function_id=0 (f_a) as having ran
        assert_eq!(State::Blocked, state.get_state(0), "f_a should be Blocked");
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

        assert!(state.next_job().is_none());
    }

    #[test]
    fn next_works() {
        let mut state = RunState::new(test_functions(), 1);

        // Put 0 on the blocked/ready
        state.inputs_now_full(0);

        assert_eq!(state.next_job().unwrap().function_id, 0);
    }

    #[test]
    fn inputs_ready_makes_ready() {
        let mut state = RunState::new(test_functions(), 1);

        // Put 0 on the blocked/ready list depending on blocked status
        state.inputs_now_full(0);

        assert_eq!(state.next_job().unwrap().function_id, 0);
    }

    #[test]
    fn blocked_is_not_ready() {
        let mut state = RunState::new(test_functions(), 1);

        // Indicate that 0 is blocked by 1
        state.set_blocked_by(1, 0);

        // Put 0 on the blocked/ready list depending on blocked status
        state.inputs_now_full(0);

        assert!(state.next_job().is_none());
    }

    #[test]
    fn unblocking_makes_ready() {
        let mut state = RunState::new(test_functions(), 1);

        // Indicate that 0 is blocked by 1
        state.set_blocked_by(1, 0);

        // Put 0 on the blocked/ready list depending on blocked status
        state.inputs_now_full(0);

        assert!(state.next_job().is_none());

        // now unblock 0 by 1
        state.unblock_senders_to(1);

        // Now function with id 0 should be ready and served up by next
        assert_eq!(state.next_job().unwrap().function_id, 0);
    }

    #[test]
    fn unblocking_doubly_blocked_functions_not_ready() {
        let mut state = RunState::new(test_functions(), 1);

        // Indicate that 0 is blocked by 1 and 2
        state.set_blocked_by(1, 0);
        state.set_blocked_by(2, 0);

        // Put 0 on the blocked/ready list depending on blocked status
        state.inputs_now_full(0);

        assert!(state.next_job().is_none());

        // now unblock 0 by 1
        state.unblock_senders_to(1);

        // Now function with id 0 should still not be ready as still blocked on 2
        assert!(state.next_job().is_none());
    }

    #[test]
    fn wont_return_too_many_jobs() {
        let mut state = RunState::new(test_functions(), 1);

        // Put 0 on the blocked/ready
        state.inputs_now_full(0);
        // Put 1 on the blocked/ready
        state.inputs_now_full(1);

        assert_eq!(state.next_job().unwrap().function_id, 0);
        assert!(state.next_job().is_none());
    }
}
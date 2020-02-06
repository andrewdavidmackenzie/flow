use std::collections::HashSet;
use std::collections::VecDeque;
use std::fmt;
use std::sync::Arc;

use flow_impl::Implementation;
use log::{debug, error};
use multimap::MultiMap;
use serde_json::Value;

use crate::debugger::Debugger;
use crate::function::Function;
use crate::metrics::Metrics;
use crate::output_connection::OutputConnection;

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
    pub job_id: usize,
    pub function_id: usize,
    pub implementation: Arc<dyn Implementation>,
    pub input_set: Vec<Vec<Value>>,
    pub destinations: Vec<OutputConnection>,
}

#[derive(Debug)]
pub struct Output {
    pub job_id: usize,
    pub function_id: usize,
    pub input_values: Vec<Vec<Value>>,
    pub result: (Option<Value>, bool),
    pub destinations: Vec<OutputConnection>,
    pub error: Option<String>,
}

///
/// RunList is a structure that maintains the state of all the functions in the currently
/// executing flow.
///
/// A function maybe blocking multiple others trying to send data to it.
/// Those others maybe blocked trying to send to multiple different function.
///
/// The Semantics of a Flow's RunState
/// ==================================
/// The semantics of the state of each function in a flow and the flow over are described here
/// and the tests of the struct attempt to reproduce and confirm as many of them as is possible
///
/// Terminology
/// ===========
/// * function        - an entry in the manifest and the flow graph that may take inputs, will execute an
///                     implementation on a Job and may produce an Output
/// * input           - a function may have 0 or more inputs that accept values required for it's execution
/// * implementation  - the code that is run, accepting 0 or more input values performing some calculations
///                     and possibly producing an output value. One implementation can be used by multiple
///                     functions in a flow
/// * destinations    - a set of other functions and their specific inputs that a function is connected
///                     to and hence where the output value is sent when execution is completed
/// * job             - a job is the bundle of information necessary to execute. It consists of the
///                     function's id, the input values, the implementation to run, and the destinations
///                     to send the output value to
/// * execution       - the act of running an implementation on the input values to produce an output
/// * output          - a function when ran produces an output. The output contains the id of the function
///                     that was ran, the input values (for debugging), the result (optional value plus
///                     an indicator if the function wishes to be ran again when ready), the destinations
///                     to send any value to and an optional error string.
///
/// Start-up
/// ==============
/// At start-up all functions are initialized. For each of the functions inputs their
/// init_inputs() function will be called, meaning that some inputs may be initialized (filled):
///    - Other functions that send to these initialized inputs will be blocked initially
/// If all inputs are full then the Function maybe able to run, depending on existence of blocks on
/// other functions it sends to.
///
/// One-time Execution or Stopping repetitive execution
/// ===================================================
/// It may make sense for some functions to only be ran once, or to stop being executed repeatedly
/// as some point, so each implementation when run returns a "run again" flag to indicate this.
/// An example of functions that may decide to stop running are:
/// - args: produces arguments from the command line execution of a flow once at start-up
/// - readline: read a line of input from standard input, until End-of-file (EOF) is detected.
///   If this was not done, then the flow would never stop running as the readline function would
///   always be re-run and waiting for more input, but none would ever be received after EOF.
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
/// Unused Functions
/// ================
/// If a pure function has an output but it is not used (not connected to any input) then the function
/// should have no affect on the execution of the flow and the optimizer may remove it and all
/// connections to its input. That in turn may affect other functions which can be removed, until
/// there are no more left to remove.
/// Thus at run-time, a pure function with it's output unused is not expected and no special handling
/// of that case is taken. If a manifest is read where a pure function has no destinations, then
/// it will be run (when it received inputs) and it's output discarded.
/// That is sub-optimal execution but no errors should result. Hence the role of the optimizer at
/// compile time.
/// Tests: pure_function_no_destinations()
///
/// Unconnected inputs
/// ==================
/// If a function's output is used but one or more of it's inputs is unconnected, then the compiler
/// should throw an error. If for some reason an alternative compiler did not reject this and
/// generated a manifest with no other function sending to that input, then at run-time that functions
/// inputs will never be full and the function will never run. This could produce some form of deadlock
/// or incomplete execution, but it should not produce any run-time error.
/// A run-time is within it's rights to discard this function, and then potentially other functions
/// connected to it's output and other inputs until no more can be removed.
/// This run-time does not do that, in order to keep things simple and start-up time to a minimum.
///
/// Runtime Rules
/// =============
/// * A function won't be run until all of its inputs are ready and all of the inputs on other
///   functions it sends to are empty and able to receive its output. (See 'Loops' below)
/// * An input maybe initialized at start-up once by a "Once" input initializer
/// * An input maybe initialized after each run by a "Constant" input initializer that ensures that
///   the same value always re-fills the input
/// * When one or more inputs on other functions that a function sends to are full, then the sending
///   function will be in the blocked state
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
/// Constant Initializers
/// =====================
/// A function input may have a "ConstantInitializer" on it to continually re-fill the input.
/// After the functions runs this is run and the input refilled.
///
/// Loops
/// =====
/// A function may send to itself.
/// A function sending to itself will not create any blocks, and will not be marked as blocked
/// due to the loop, and thus such deadlocks avoided.
///
/// Blocks on other senders due to Constant Initializers and Loops
/// ==============================================================
/// After a funtion runs, its ConstantInitializers are ran, and outputs (possibly to itself) are
/// sent, before determining that other functions sending to it should unblocked.
/// This, the initializers and loops to it's inputs have priority and the input(s) will be refilled
/// but another function wishing to send to it, and blocked, is NOT yet unblocked.
/// TODO TEST
///
/// Parallel Execution of Jobs
/// ==========================
/// Multiple functions (jobs) may execute in parallel, providing there is no data dependency
/// preventing it. Example dependencies:
///   * a function lacks an input and needs to get it from another function that has not completed
///   * a function cannot run, as it's output iss connected to another function's input that is full
/// Respecting this rule, a RunTime can dispatch as many Jobs in parallel as it desires. This one
/// takes the parameter max_jobs on RunState::new() to specify the maximum number of jobs that are
/// launched in parallel. The minimum value for this is 1
///
pub struct RunState {
    functions: Vec<Function>,
    blocked: HashSet<usize>,
    // blocked: HashSet<function_id>
    blocks: VecDeque<(usize, usize, usize)>,
    // blocking: Vec<(blocking_id, blocking_io_number, blocked_id)>
    ready: VecDeque<usize>,
    // ready: Vec<function_id>
    running: MultiMap<usize, usize>,
    // running: MultiMap<function_id, job_id>
    jobs_sent: usize,
    // number of jobs sent to date
    max_jobs: usize,
    // limit on the number of jobs to allow to run in parallel
}

impl RunState {
    pub fn new(functions: Vec<Function>, max_jobs: usize) -> Self {
        RunState {
            functions,
            blocked: HashSet::<usize>::new(),
            blocks: VecDeque::<(usize, usize, usize)>::new(),
            ready: VecDeque::<usize>::new(),
            running: MultiMap::<usize, usize>::new(),
            jobs_sent: 0,
            max_jobs,
        }
    }

    #[cfg(feature = "debugger")]
    /*
        Reset all values back to inital ones to enable debugging from scracth
    */
    fn reset(&mut self) {
        for function in &mut self.functions {
            function.reset()
        };
        self.blocked.clear();
        self.blocks.clear();
        self.ready.clear();
        self.running.clear();
        self.jobs_sent = 0;
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
        self.reset();

        let mut inputs_ready_list = Vec::<usize>::new();

        for function in &mut self.functions {
            debug!("\tInitializing Function #{} '{}'", function.id(), function.name());
            function.init_inputs(true);
            if function.inputs_full() {
                inputs_ready_list.push(function.id());
            }
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
        let mut blocks = VecDeque::<(usize, usize, usize)>::new();
        let mut blocked = HashSet::<usize>::new();

        debug!("Creating any initial block entries that are needed");

        for source_function in &self.functions {
            let source_id;
            let destinations;
            let source_has_inputs_full;
            {
                source_id = source_function.id();
                source_has_inputs_full = source_function.inputs_full();
                destinations = source_function.output_destinations().clone();
            }
            // (_output_path, destination_id, io_number, _destination_path)
            for destination in destinations {
                if destination.function_id != source_id { // don't block yourself!
                    let destination_function = self.get(destination.function_id);
                    if destination_function.input_full(destination.io_number) {
                        debug!("\tAdded block between #{} <-- #{}", destination.function_id, source_id);
                        blocks.push_back((destination.function_id,
                                          destination.io_number, source_id));
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
            } else if self.running.contains_key(&function_id) {
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
        let function_state = self.get_state(function_id);
        let mut output = format!("\tState: {:?}\n", function_state);

        if function_state == State::Running {
            output.push_str(&format!("\t\tJob Numbers Running: {:?}\n",
                                     self.running.get_vec(&function_id).unwrap()));
        }

        for (blocking, blocking_io_number, blocked) in &self.blocks {
            if *blocked == function_id {
                output.push_str(&format!("\t\tBlocked #{} --> Blocked by #{}:{}\n",
                                         blocked, blocking, blocking_io_number));
            } else if *blocking == function_id {
                output.push_str(&format!("\t\tBlocking #{}:{} <-- Blocked #{}\n",
                                         blocking, blocking_io_number, blocked));
            }
        }

        output
    }

    pub fn get(&self, id: usize) -> &Function {
        &self.functions[id]
    }

    pub fn get_mut(&mut self, id: usize) -> &mut Function {
        &mut self.functions[id]
    }

    /*
        Return the next job ready to be run, if there is one and there are not
        too many jobs already running
    */
    pub fn next_job(&mut self) -> Option<Job> {
        if self.ready.is_empty() || self.number_jobs_running() >= self.max_jobs {
            return None;
        }

        // create a job for the function_id at the head of the ready list
        let function_id = *self.ready.get(0).unwrap();
        let (job, can_create_more_jobs) = self.create_job(function_id);

        // only remove it from the ready list if its inputs are not still full
        if !can_create_more_jobs {
            self.ready.remove(0);
        }

        Some(job)
    }

    /*
        Track the number of jobs sent to date
    */
    pub fn job_sent(&mut self) {
        self.jobs_sent += 1;
    }

    /*
        Given a function id, prepare a job for execution that contains the input values, the
        implementation and the destination functions the output should be sent to when done
        Return:
            - Job created
            - true if we took an input set but enough remain to create more jobs
    */
    fn create_job(&mut self, function_id: usize) -> (Job, bool) {
        let job_id = self.jobs_sent;
        debug!("Creating Job #{} for Function #{}", self.jobs_sent, function_id);

        let function = self.get_mut(function_id);

        let input_set = function.take_input_set();

        // refresh any inputs that have constant initializers
        let refilled = function.init_inputs(false);
        let all_refilled = refilled.len() == function.inputs().len();

        debug!("Job #{},  Function #{} '{}', Input set: {:?}", job_id, function_id, function.name(), input_set);

        let implementation = function.get_implementation();

        let destinations = function.output_destinations().clone();

        // create more jobs for the same function if:
        //    - it has inputs, otherwise we can generate infinite number of jobs
        //    - it does not have ONLY ConstantInitialized inputs and we have refilled them
        //    - all the inputs are still full, so we can create another job for this function
        let can_create_more_jobs = !function.inputs().is_empty() && function.inputs_full()
            && !all_refilled;

        (Job { job_id, function_id, implementation, input_set, destinations },
         can_create_more_jobs)
    }

    /*
        Take an output produced by a function and modify the runlist accordingly
        If other functions were blocked trying to send to this one - we can now unblock them
        as it has consumed it's inputs and they are free to be sent to again.

        Then take the output and send it to all destination IOs on different function it should be
        sent to, marking the source function as blocked because those others must consume the output
        if those other function have all their inputs, then mark them accordingly.
    */
    pub fn process_output(&mut self, metrics: &mut Metrics, output: Output, debugger: &mut Option<Debugger>) {
        match output.error {
            None => {
                let output_value = output.result.0;
                let source_can_run_again = output.result.1;

                // if it produced an output value
                if let Some(output_v) = output_value {
                    debug!("\tProcessing output value '{}' from Job #{}", output_v, output.job_id);

                    //output_route, destination_id, io_number, _destination_path)
                    for destination in &output.destinations {
                        let output_value = output_v.pointer(&destination.subpath).unwrap();
                        self.send_value(output.function_id,
                                        &destination.subpath,
                                        destination.function_id,
                                        destination.io_number, output_value, metrics, debugger);
                    }
                }

                // if it wants to run again, and after possibly refreshing any constant inputs, it can
                // (it's inputs are ready) then add back to the Ready list
                if source_can_run_again {
                    let (refilled, full) = self.refill_inputs(output.function_id);
                    if full {
                        self.inputs_now_full(output.function_id);
                    } else {
                        // unblock senders blocked trying to send to this functions empty inputs
                        self.unblock_senders_to(output.function_id, refilled);
                    }
                }
            }
            Some(_) => {
                match debugger {
                    None => error!("Error in Job execution:\n{:#?}", output),
                    Some(debugger) => debugger.panic(&self, output)
                }
            }
        }
    }

    /*
        Send a value produced as part of an output of running a job to a destination function on
        a specific input, update the metrics and potentially enter the debugger
    */
    fn send_value(&mut self, source_id: usize, output_route: &str, destination_id: usize, io_number: usize,
                  output_value: &Value, metrics: &mut Metrics, debugger: &mut Option<Debugger>) {
        let block;
        let full;

        debug!("\t\tJob #{} sending value '{}' via output route '{}' to Function #{}:{}",
               source_id, output_value, output_route, destination_id, io_number);

        if let Some(ref mut debugger) = debugger {
            debugger.check_prior_to_send(self, source_id, output_route,
                                         &output_value, destination_id, io_number);
        }

        {
            let destination = self.get_mut(destination_id);

            destination.write_input(io_number, output_value);

            #[cfg(feature = "metrics")]
                metrics.increment_outputs_sent();

            block = destination.input_full(io_number);

            // for the case when a function is sending to itself, delay determining if it should
            // be in the blocked or ready lists until it has sent all it's other outputs
            // as it might be blocked by another function.
            full = destination.inputs_full() && (source_id != destination_id);
        }

        if block {
            self.create_block(destination_id, io_number, source_id, debugger);
        }

        if full {
            self.inputs_now_full(destination_id);
        }
    }

    /*
        Refresh any inputs that have initializers on them
    */
    fn refill_inputs(&mut self, id: usize) -> (Vec<usize>, bool) {
        let function = self.get_mut(id);

        // refresh any constant inputs it may have
        let refilled = function.init_inputs(false);

        (refilled, function.inputs_full())
    }

    /*
        Removes any entry from the running list where k=function_id AND v=job_id
        as there maybe more than one job running with function_id
    */
    pub fn job_done(&mut self, output: &Output) {
        self.running.retain(|&k, &v| k != output.function_id || v != output.job_id);
    }

    pub fn start(&mut self, job: &Job) {
        self.running.insert(job.function_id, job.job_id);
    }

    // See if there is any tuple in the vector where the second (blocked_id) is the one we're after
    fn is_blocked(&self, id: usize) -> bool {
        for &(_blocking_id, _io_number, blocked_id) in &self.blocks {
            if blocked_id == id {
                return true;
            }
        }
        false
    }

    #[cfg(feature = "debugger")]
    pub fn get_output_blockers(&self, id: usize) -> Vec<(usize, usize)> {
        let mut blockers = vec!();

        for &(blocking_id, blocking_io_number, blocked_id) in &self.blocks {
            if blocked_id == id {
                blockers.push((blocking_id, blocking_io_number));
            }
        }

        blockers
    }

    pub fn number_jobs_running(&self) -> usize {
        let mut num_running_jobs = 0;
        for (_, vector) in self.running.iter_all() {
            num_running_jobs += vector.len()
        };
        num_running_jobs
    }

    pub fn number_jobs_ready(&self) -> usize {
        self.ready.len()
    }

    /*
        An input blocker is another function that is the only function connected to an empty input
        of target function, and which is not ready to run, hence target function cannot run.
    */
    #[cfg(feature = "debugger")]
    pub fn get_input_blockers(&self, target_id: usize) -> Vec<(usize, usize)> {
        let mut input_blockers = vec!();
        let target_function = self.get(target_id);

        // for each empty input of the target function
        for (target_io, input) in target_function.inputs().iter().enumerate() {
            if input.is_empty() {
                let mut senders = Vec::<(usize, usize)>::new();

                // go through all functions to see if sends to the target function on input
                for sender_function in &self.functions {
                    // if the sender function is not ready to run
                    if !self.ready.contains(&sender_function.id()) {

                        // for each output route of sending function, see if it is sending to the target function and input
                        //(ref _output_route, destination_id, io_number, _destination_path)
                        for destination in sender_function.output_destinations() {
                            if (destination.function_id == target_id) &&
                                (destination.io_number == target_io) {
                                senders.push((sender_function.id(), target_io));
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
            debug!("\t\t\tFunction #{} not blocked on output, so added to 'Ready' list", id);
            self.mark_ready(id);
        }
    }

    fn mark_ready(&mut self, id: usize) {
        if !self.ready.contains(&id) {
            self.ready.push_back(id);
        }
    }

    pub fn jobs(&self) -> usize {
        self.jobs_sent
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
    fn unblock_senders_to(&mut self, blocker_id: usize, refilled_inputs: Vec<usize>) {
        if !self.blocks.is_empty() {
            let mut unblocked_list = vec!();

            for &(blocking_id, blocking_io_number, blocked_id) in &self.blocks {
                if (blocking_id == blocker_id) && !refilled_inputs.contains(&blocking_io_number) {
                    debug!("\t\tBlock removed #{}:{} <-- #{}", blocking_id, blocking_io_number, blocked_id);
                    unblocked_list.push(blocked_id);
                }
            }

            // retain all  blocks unaffected by removing this one
            self.blocks.retain(|&(blocking_id, blocking_io_number, _blocked_id)|
                !((blocking_id == blocker_id) && !refilled_inputs.contains(&blocking_io_number))
            );

            // see if the functions unblocked should no be made ready.
            // Note, they could be blocked on other functions apart from the the one that just unblocked
            for unblocked in unblocked_list {
                if self.blocked.contains(&unblocked) && !self.is_blocked(unblocked) {
                    debug!("\t\t\tFunction #{} removed from 'blocked' list", unblocked);
                    self.blocked.remove(&unblocked);

                    if self.get(unblocked).inputs_full() {
                        debug!("\t\t\tFunction #{} has inputs ready, so added to 'ready' list", unblocked);
                        self.mark_ready(unblocked);
                    }
                }
            }
        }
    }

    /*
        Create a 'block" indicating that function 'blocked_id' cannot run as it has an output
        destination to an input on function 'blocking_id' that is already full.

        Avoid deadlocks caused by a function blocking itself
    */
    fn create_block(&mut self, blocking_id: usize, blocking_io_number: usize,
                    blocked_id: usize, debugger: &mut Option<Debugger>) {
        if blocked_id != blocking_id {
            debug!("\t\t\tProcess #{}:{} <-- Process #{} blocked", &blocking_id,
                   &blocking_io_number, &blocked_id);

            if !self.blocks.contains(&(blocking_id, blocking_io_number, blocked_id)) {
                self.blocks.push_back((blocking_id, blocking_io_number, blocked_id));
                if let Some(ref mut debugger) = debugger {
                    debugger.check_on_block_creation(self, blocking_id, blocking_io_number, blocked_id);
                }
            }
        }
    }
}

#[cfg(any(feature = "logging", feature = "debugger"))]
impl fmt::Display for RunState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "RunState:\n")?;
        write!(f, "   Processes: {}\n", self.functions.len())?;
        write!(f, "        Jobs: {}\n", self.jobs_sent)?;
        write!(f, "     Blocked: {:?}\n", self.blocked)?;
        write!(f, "      Blocks: {:?}\n", self.blocks)?;
        write!(f, "       Ready: {:?}\n", self.ready)?;
        write!(f, "     Running: {:?}\n", self.running)
    }
}

#[cfg(test)]
mod test {
    use serde_json::json;

    use crate::debug_client::{DebugClient, Event, Response};
    use crate::debug_client::{Command, Param};
    use crate::function::Function;
    use crate::input::Input;
    use crate::input::InputInitializer::OneTime;
    use crate::input::OneTimeInputInitializer;
    use crate::output_connection::OutputConnection;
    use crate::run_state;

    use super::Output;

    // Helpers
    struct TestDebugClient {}

    impl DebugClient for TestDebugClient {
        fn init(&self) {}

        fn get_command(&self, _job_number: Option<usize>) -> Command {
            Command::Step(Some(run_state::test::Param::Numeric(1)))
        }

        fn send_event(&self, _event: Event) {}

        fn send_response(&self, _response: Response) {}
    }

    fn test_debug_client() -> &'static dyn DebugClient {
        &TestDebugClient {}
    }

    fn test_function_a_to_b_not_init() -> Function {
        let out_conn = OutputConnection::new("".to_string(), 1, 0, None);
        Function::new("fA".to_string(), // name
                      "/context/fA".to_string(),
                      "/test".to_string(),
                      vec!(Input::new(1, &None, false)),
                      0,
                      &vec!(out_conn), false) // outputs to fB:0
    }

    fn test_function_a_to_b() -> Function {
        let out_conn = OutputConnection::new("".to_string(), 1, 0, None);
        Function::new("fA".to_string(), // name
                      "/context/fA".to_string(),
                      "/test".to_string(),
                      vec!(Input::new(1,
                                      &Some(OneTime(OneTimeInputInitializer { once: json!(1) })),
                                      false)),
                      0,
                      &vec!(out_conn), false) // outputs to fB:0
    }

    fn test_function_a_init() -> Function {
        Function::new("fA".to_string(), // name
                      "/context/fA".to_string(),
                      "/test".to_string(),
                      vec!(Input::new(1,
                                      &Some(OneTime(OneTimeInputInitializer { once: json!(1) })),
                                      false)),
                      0,
                      &vec!(), false)
    }

    fn test_function_b_not_init() -> Function {
        Function::new("fB".to_string(), // name
                      "/context/fB".to_string(),
                      "/test".to_string(),
                      vec!(Input::new(1, &None, false)),
                      1,
                      &vec!(), false)
    }

    fn test_function_b_init() -> Function {
        Function::new("fB".to_string(), // name
                      "/context/fB".to_string(),
                      "/test".to_string(),
                      vec!(Input::new(1,
                                      &Some(OneTime(OneTimeInputInitializer { once: json!(1) })),
                                      false)),
                      1,
                      &vec!(), false)
    }

    fn test_output(source_function_id: usize, dest_function_id: usize) -> Output {
        let out_conn = OutputConnection::new("".to_string(), dest_function_id, 0, None);
        Output {
            job_id: 1,
            function_id: source_function_id,
            input_values: vec!(vec!(json!(1))),
            result: (Some(json!(1)), true),
            destinations: vec!(out_conn),
            error: None,
        }
    }

    fn error_output(source_function_id: usize, dest_function_id: usize) -> Output {
        let out_conn = OutputConnection::new("".to_string(), dest_function_id, 0, None);
        Output {
            job_id: 1,
            function_id: source_function_id,
            input_values: vec!(vec!(json!(1))),
            result: (None, false),
            destinations: vec!(out_conn),
            error: Some("Some error occurred".to_string()),
        }
    }

    mod general_run_state_tests {
        use std::collections::HashSet;

        use super::super::RunState;
        use super::super::State;

        #[cfg(any(feature = "logging", feature = "debugger"))]
        #[test]
        fn run_state_can_display() {
            let f_a = super::test_function_a_to_b();
            let f_b = super::test_function_b_not_init();
            let functions = vec!(f_a, f_b);
            let mut state = RunState::new(functions, 1);

            state.init();

            println!("{}", state);
        }

        #[cfg(any(feature = "debugger"))]
        #[test]
        fn debugger_can_display_run_state() {
            let f_a = super::test_function_a_to_b();
            let f_b = super::test_function_b_init();
            let functions = vec!(f_b, f_a);
            let mut state = RunState::new(functions, 1);

            // Event
            state.init();

            // Test
            assert_eq!(2, state.num_functions(), "There should be 2 functions");
            assert_eq!(State::Blocked, state.get_state(0), "f_a should be in Blocked state");
            assert_eq!(State::Ready, state.get_state(1), "f_b should be Ready");
            assert_eq!(1, state.number_jobs_ready(), "There should be 1 job running");
            let mut blocked = HashSet::new();
            blocked.insert(0);

            // Test
            assert_eq!(&blocked, state.get_blocked(), "Function with ID = 1 should be in 'blocked' list");
            state.display_state(0);
            state.display_state(1);

            // Event
            let job = state.next_job().unwrap();
            state.start(&job);

            // Test
            assert_eq!(State::Running, state.get_state(1), "f_b should be Running");
            assert_eq!(1, state.number_jobs_running(), "There should be 1 job running");
            state.display_state(1);
        }

        #[test]
        fn jobs_sent_zero_at_init() {
            let mut state = RunState::new(vec!(), 1);
            state.init();
            assert_eq!(0, state.jobs(), "At init jobs() should be 0");
        }

        #[test]
        fn jobs_sent_increases() {
            let mut state = RunState::new(vec!(), 1);
            state.init();
            state.job_sent();
            assert_eq!(1, state.jobs(), "jobs() should have incremented");
        }
    }

    /********************************* State Transition Tests *********************************/
    mod state_transitions {
        use serde_json::json;

        use crate::debugger::Debugger;
        use crate::function::Function;
        use crate::input::{ConstantInputInitializer, OneTimeInputInitializer};
        use crate::input::Input;
        use crate::input::InputInitializer::{Constant, OneTime};
        use crate::metrics::Metrics;
        use crate::output_connection::OutputConnection;

        use super::super::Output;
        use super::super::RunState;
        use super::super::State;
        use super::test_debug_client;

        #[test]
        fn to_ready_1_on_init() {
            let f_a = super::test_function_a_to_b();
            let f_b = super::test_function_b_not_init();
            let functions = vec!(f_a, f_b);
            let mut state = RunState::new(functions, 1);

            // Event
            state.init();

            // Test
            assert_eq!(State::Ready, state.get_state(0), "f_a should be Ready");
            assert_eq!(State::Waiting, state.get_state(1), "f_b should be waiting for input");
        }

        #[test]
        fn input_blocker() {
            let f_a = super::test_function_a_to_b_not_init();
            let f_b = super::test_function_b_not_init();
            let functions = vec!(f_a, f_b);
            let mut state = RunState::new(functions, 1);

            // Event
            state.init();

            // Test
            assert_eq!(State::Waiting, state.get_state(0), "f_a should be waiting for input");
            assert_eq!(State::Waiting, state.get_state(1), "f_b should be waiting for input");
            #[cfg(feature = "debugger")]
            assert!(state.get_input_blockers(1).contains(&(0, 0)), "f_b should be waiting for input from f_a")
        }

        #[test]
        fn to_ready_2_on_init() {
            let f_a = super::test_function_a_to_b();
            let f_b = super::test_function_b_not_init();
            let functions = vec!(f_a, f_b);
            let mut state = RunState::new(functions, 1);

            // Event
            state.init();

            // Test
            assert_eq!(State::Ready, state.get_state(0), "f_a should be Ready");
        }

        #[test]
        fn to_ready_3_on_init() {
            let f_a = super::test_function_a_init();
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
            let f_a = super::test_function_a_to_b();
            let f_b = super::test_function_b_init();
            let functions = vec!(f_b, f_a);
            let mut state = RunState::new(functions, 1);

            // Event
            state.init();

            // Test
            assert_eq!(State::Ready, state.get_state(1), "f_b should be Ready");
            assert_eq!(State::Blocked, state.get_state(0), "f_a should be in Blocked state");
            #[cfg(feature = "debugger")]
            assert!(state.get_output_blockers(0).contains(&(1, 0)), "f_a should be blocked by f_b, input 0");
        }

        #[test]
        fn to_waiting_on_init() {
            let f_a = Function::new("fA".to_string(), // name
                                    "/context/fA".to_string(),
                                    "/test".to_string(),
                                    vec!(Input::new(1, &None, false)),
                                    0,
                                    &vec!(), false);
            let functions = vec!(f_a);
            let mut state = RunState::new(functions, 1);

            // Event
            state.init();

            // Test
            assert_eq!(State::Waiting, state.get_state(0), "f_a should be Waiting");
        }

        #[test]
        fn ready_to_running_on_next() {
            let f_a = super::test_function_a_init();
            let functions = vec!(f_a);
            let mut state = RunState::new(functions, 1);
            state.init();
            assert_eq!(State::Ready, state.get_state(0), "f_a should be Ready");

            // Event
            let job = state.next_job().unwrap();
            assert_eq!(0, job.function_id, "next_job() should return function_id = 0");
            state.start(&job);

            // Test
            assert_eq!(State::Running, state.get_state(0), "f_a should be Running");
        }

        #[test]
        fn unready_not_to_running_on_next() {
            let f_a = Function::new("fA".to_string(), // name
                                    "/context/fA".to_string(),
                                    "/test".to_string(),
                                    vec!(Input::new(1, &None, false)),
                                    0,
                                    &vec!(), false);
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
            let f_a = super::test_function_a_to_b();
            let f_b = super::test_function_b_init();
            let functions = vec!(f_b, f_a); // NOTE the order!
            let mut state = RunState::new(functions, 1);
            let mut metrics = Metrics::new(2);
            let mut debugger = Some(Debugger::new(test_debug_client()));
            state.init();
            assert_eq!(State::Ready, state.get_state(1), "f_b should be Ready");
            assert_eq!(State::Blocked, state.get_state(0), "f_a should be in Blocked state, by f_b");
            assert_eq!(1, state.next_job().unwrap().function_id, "next() should return function_id=1 (f_b) for running");

            // Event
            let output = super::test_output(1, 0);
            state.process_output(&mut metrics, output, &mut debugger);

            // Test
            assert_eq!(State::Ready, state.get_state(0), "f_a should be Ready");
        }

        #[test]
        fn process_error_output() {
            let f_a = super::test_function_a_init();
            let f_b = super::test_function_b_not_init();
            let functions = vec!(f_a, f_b);
            let mut state = RunState::new(functions, 1);
            let mut metrics = Metrics::new(2);
            let mut debugger = Some(Debugger::new(test_debug_client()));

            state.init();
            let output = super::error_output(0, 1);
            state.process_output(&mut metrics, output, &mut debugger);

            assert_eq!(State::Waiting, state.get_state(1), "f_b should be Waiting");
        }

        #[test]
        fn running_to_ready_on_done() {
            let f_a = Function::new("fA".to_string(), // name
                                    "/context/fA".to_string(),
                                    "/test".to_string(),
                                    vec!(Input::new(1,
                                                    &Some(Constant(ConstantInputInitializer { constant: json!(1) })),
                                                    false)),
                                    0,
                                    &vec!(), false);
            let functions = vec!(f_a);
            let mut state = RunState::new(functions, 1);
            let mut metrics = Metrics::new(1);
            let mut debugger = Some(Debugger::new(test_debug_client()));
            state.init();
            assert_eq!(State::Ready, state.get_state(0), "f_a should be Ready");
            let job = state.next_job().unwrap();
            assert_eq!(0, job.function_id, "next() should return function_id = 0");
            state.start(&job);

            assert_eq!(State::Running, state.get_state(0), "f_a should be Running");

            // Event
            let output = Output {
                job_id: 1,
                function_id: 0,
                input_values: vec!(vec!(json!(1))),
                result: (None, true),
                destinations: vec!(),
                error: None,
            };
            state.process_output(&mut metrics, output, &mut debugger);

            // Test
            assert_eq!(State::Ready, state.get_state(0), "f_a should be Ready again");
        }

        // Done: it has one input or more empty, to it can't run
        #[test]
        fn running_to_waiting_on_done() {
            let f_a = super::test_function_a_init();
            let functions = vec!(f_a);
            let mut state = RunState::new(functions, 1);
            let mut metrics = Metrics::new(1);
            let mut debugger = Some(Debugger::new(test_debug_client()));
            state.init();
            assert_eq!(State::Ready, state.get_state(0), "f_a should be Ready");
            let job = state.next_job().unwrap();
            assert_eq!(0, job.function_id, "next() should return function_id = 0");
            state.start(&job);

            assert_eq!(State::Running, state.get_state(0), "f_a should be Running");

            // Event
            let output = Output {
                job_id: 0,
                function_id: 0,
                input_values: vec!(vec!(json!(1))),
                result: (None, true),
                destinations: vec!(),
                error: None,
            };
            state.job_done(&output);
            state.process_output(&mut metrics, output, &mut debugger);

            // Test
            assert_eq!(State::Waiting, state.get_state(0), "f_a should be Waiting again");
        }

        // Done: at least one destination input is full, so can't run  running_to_blocked_on_done
        #[test]
        fn running_to_blocked_on_done() {
            let out_conn = OutputConnection::new("".to_string(), 1, 0, None);
            let f_a = Function::new("fA".to_string(), // name
                                    "/context/fA".to_string(),
                                    "/test".to_string(),
                                    vec!(Input::new(1,
                                                    &Some(Constant(ConstantInputInitializer { constant: json!(1) })),
                                                    false)),
                                    0,
                                    &vec!(out_conn), false); // outputs to fB:0
            let f_b = Function::new("fB".to_string(), // name
                                    "/context/fB".to_string(),
                                    "/test".to_string(),
                                    vec!(Input::new(1, &None, false)),
                                    1,
                                    &vec!(), false);
            let functions = vec!(f_a, f_b);
            let mut state = RunState::new(functions, 1);
            let mut metrics = Metrics::new(1);
            let mut debugger = Some(Debugger::new(test_debug_client()));
            state.init();

            assert_eq!(State::Ready, state.get_state(0), "f_a should be Ready");

            let job = state.next_job().unwrap();
            assert_eq!(0, job.function_id, "next() should return function_id=0 (f_a) for running");
            state.start(&job);

            assert_eq!(State::Running, state.get_state(0), "f_a should be Running");

            // Event
            let output = super::test_output(0, 1);
            state.process_output(&mut metrics, output, &mut debugger);

            // Test f_a should transition to Blocked on f_b
            assert_eq!(State::Blocked, state.get_state(0), "f_a should be Blocked");
        }

        #[test]
        fn waiting_to_ready_on_input() {
            let f_a = Function::new("fA".to_string(), // name
                                    "/context/fA".to_string(),
                                    "/test".to_string(),
                                    vec!(Input::new(1, &None, false)),
                                    0,
                                    &vec!(), false);
            let out_conn = OutputConnection::new("".into(), 0, 0, None);
            let f_b = Function::new("fB".to_string(), // name
                                    "/context/fB".to_string(),
                                    "/test".to_string(),
                                    vec!(Input::new(1, &None, false)),
                                    1,
                                    &vec!(out_conn), false);
            let functions = vec!(f_a, f_b);
            let mut state = RunState::new(functions, 1);
            let mut metrics = Metrics::new(1);
            let mut debugger = Some(Debugger::new(test_debug_client()));
            state.init();
            assert_eq!(State::Waiting, state.get_state(0), "f_a should be Waiting");

            // Event run f_b which will send to f_a
            let output = super::test_output(1, 0);
            state.process_output(&mut metrics, output, &mut debugger);

            // Test
            assert_eq!(State::Ready, state.get_state(0), "f_a should be Ready");
        }

        #[test]
        fn waiting_to_blocked_on_input() {
            let f_a = super::test_function_a_to_b_not_init();
            let out_conn = OutputConnection::new("".into(), 0, 0, None);
            let f_b = Function::new("fB".to_string(), // name
                                    "/context/fB".to_string(),
                                    "/test".to_string(),
                                    vec!(Input::new(1,
                                                    &Some(Constant(ConstantInputInitializer { constant: json!(1) })),
                                                    false)),
                                    1,
                                    &vec!(out_conn), false);
            let functions = vec!(f_a, f_b);
            let mut state = RunState::new(functions, 1);
            let mut metrics = Metrics::new(1);
            let mut debugger = Some(Debugger::new(test_debug_client()));
            state.init();

            assert_eq!(State::Ready, state.get_state(1), "f_b should be Ready");
            assert_eq!(State::Waiting, state.get_state(0), "f_a should be in Waiting");

            // Event run f_b which will send to f_a, but will block f_a due to initialize
            let output = super::test_output(1, 0);
            state.process_output(&mut metrics, output, &mut debugger);

            // Test
            assert_eq!(State::Blocked, state.get_state(0), "f_a should be Blocked");
        }

        /*
            This tests that if a function that has a loop back sending to itself, runs the firts time
            due to a OnceInitializer, that after running it sends output back to itself and is ready
            (not waiting for an input from elsewhere and no deadlock due to blocking itself occurs
        */
        #[test]
        fn not_block_on_self() {
            let out_conn1 = OutputConnection::new("".to_string(), 0, 0, None);
            let out_conn2 = OutputConnection::new("".to_string(), 1, 0, None);
            let f_a = Function::new("fA".to_string(), // name
                                    "/context/fA".to_string(),
                                    "/test".to_string(),
                                    vec!(Input::new(1,
                                                    &Some(OneTime(OneTimeInputInitializer { once: json!(1) })),
                                                    false)),
                                    0,
                                    &vec!(
                                        out_conn1, // outputs to self:0
                                        out_conn2 // outputs to f_b:0
                                    ), false);
            let f_b = Function::new("fB".to_string(), // name
                                    "/context/fB".to_string(),
                                    "/test".to_string(),
                                    vec!(Input::new(1, &None, false)),
                                    1,
                                    &vec!(), false);
            let functions = vec!(f_a, f_b); // NOTE the order!
            let mut state = RunState::new(functions, 1);
            let mut metrics = Metrics::new(2);
            let mut debugger = Some(Debugger::new(test_debug_client()));
            state.init();
            assert_eq!(State::Ready, state.get_state(0), "f_a should be Ready");
            assert_eq!(State::Waiting, state.get_state(1), "f_b should be in Waiting");

            assert_eq!(0, state.next_job().unwrap().function_id, "next() should return function_id=0 (f_a) for running");

            let out_conn1 = OutputConnection::new("".into(), 0, 0, None);
            let out_conn2 = OutputConnection::new("".into(), 1, 0, None);
            // Event: run f_a
            let output = Output {
                job_id: 0,
                function_id: 0,
                input_values: vec!(vec!(json!(1))),
                result: (Some(json!(1)), true),
                destinations: vec!(out_conn1, out_conn2),
                error: None,

            };
            state.process_output(&mut metrics, output, &mut debugger);

            // Test
            assert_eq!(State::Ready, state.get_state(1), "f_b should be Ready");
            assert_eq!(State::Blocked, state.get_state(0), "f_a should be Blocked on f_b");

            assert_eq!(1, state.next_job().unwrap().function_id, "next() should return function_id=1 (f_b) for running");

            // Event: Run f_b
            let output = Output {
                job_id: 1,
                function_id: 1,
                input_values: vec!(vec!(json!(1))),
                result: (None, true),
                destinations: vec!(),
                error: None,

            };
            state.process_output(&mut metrics, output, &mut debugger);

            // Test
            assert_eq!(State::Ready, state.get_state(0), "f_a should be Ready");
        }
    }

    /****************************** Miscelaneous tests **************************/
    mod functional_tests {
        use serde_json::json;

        use crate::debugger::Debugger;
        use crate::function::Function;
        use crate::input::ConstantInputInitializer;
        use crate::input::Input;
        use crate::input::InputInitializer::Constant;
        use crate::metrics::Metrics;
        use crate::output_connection::OutputConnection;

        use super::super::Output;
        use super::super::RunState;
        use super::super::State;
        use super::test_debug_client;

        fn test_functions<'a>() -> Vec<Function> {
            let out_conn1 = OutputConnection::new("".to_string(), 1, 0, None);
            let out_conn2 = OutputConnection::new("".to_string(), 2, 0, None);
            let p0 = Function::new("p0".to_string(), // name
                                   "/context/p0".to_string(),
                                   "/test".to_string(),
                                   vec!(), // input array
                                   0,    // id
                                   &vec!(out_conn1, out_conn2) // destinations
                                   , false);    // implementation
            let p1 = Function::new("p1".to_string(),
                                   "/context/p1".to_string(),
                                   "/test".to_string(),
                                   vec!(Input::new(1, &None, false)), // inputs array
                                   1,    // id
                                   &vec!(), false);
            let p2 = Function::new("p2".to_string(),
                                   "/context/p2".to_string(),
                                   "/test".to_string(),
                                   vec!(Input::new(1, &None, false)), // inputs array
                                   2,    // id
                                   &vec!(), false);
            vec!(p0, p1, p2)
        }

        #[test]
        fn blocked_works() {
            let mut state = RunState::new(test_functions(), 1);
            let mut debugger = Some(Debugger::new(test_debug_client()));

            // Indicate that 0 is blocked by 1 on input 0
            state.create_block(1, 0, 0, &mut debugger);
            assert!(state.is_blocked(0));
        }

        #[test]
        fn get_works() {
            let state = RunState::new(test_functions(), 1);
            let got = state.get(1);
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
            let mut debugger = Some(Debugger::new(test_debug_client()));

            // Indicate that 0 is blocked by 1 on input 0
            state.create_block(1, 0, 0, &mut debugger);

            // Put 0 on the blocked/ready list depending on blocked status
            state.inputs_now_full(0);

            assert!(state.next_job().is_none());
        }

        #[test]
        fn unblocking_makes_ready() {
            let mut state = RunState::new(test_functions(), 1);
            let mut debugger = Some(Debugger::new(test_debug_client()));

            // Indicate that 0 is blocked by 1
            state.create_block(1, 0, 0, &mut debugger);

            // Put 0 on the blocked/ready list depending on blocked status
            state.inputs_now_full(0);

            assert!(state.next_job().is_none());

            // now unblock 0 by 1
            state.unblock_senders_to(1, vec!());

            // Now function with id 0 should be ready and served up by next
            assert_eq!(state.next_job().unwrap().function_id, 0);
        }

        #[test]
        fn unblocking_doubly_blocked_functions_not_ready() {
            let mut state = RunState::new(test_functions(), 1);
            let mut debugger = Some(Debugger::new(test_debug_client()));

            // Indicate that 0 is blocked by 1 and 2
            state.create_block(1, 0, 0, &mut debugger);
            state.create_block(2, 0, 0, &mut debugger);

            // Put 0 on the blocked/ready list depending on blocked status
            state.inputs_now_full(0);

            assert!(state.next_job().is_none());

            // now unblock 0 by 1
            state.unblock_senders_to(1, vec!());

            // Now function with id 0 should still not be ready as still blocked on 2
            assert!(state.next_job().is_none());
        }

        #[test]
        fn wont_return_too_many_jobs() {
            let mut state = RunState::new(test_functions(), 1);

            // Put 0 on the ready list
            state.inputs_now_full(0);
            // Put 1 on the ready list
            state.inputs_now_full(1);

            let job = state.next_job().unwrap();
            assert_eq!(0, job.function_id);
            state.start(&job);

            assert!(state.next_job().is_none());
        }

        /*
            This test checks that a function with no output destinations (even if pure and produces
            someoutput) can be executed and nothing crashes
        */
        #[test]
        fn pure_function_no_destinations() {
            let f_a = super::test_function_a_init();

            let functions = vec!(f_a);
            let mut state = RunState::new(functions, 1);
            let mut metrics = Metrics::new(1);
            let mut debugger = Some(Debugger::new(test_debug_client()));
            state.init();

            assert_eq!(state.next_job().unwrap().function_id, 0);

            // Event run f_a
            let output = Output {
                job_id: 0,
                function_id: 0,
                input_values: vec!(vec!(json!(1))),
                result: (Some(json!(1)), true),
                destinations: vec!(),
                error: None,
            };

            // Test there is no problem producing an Output when no destinations to send it to
            state.process_output(&mut metrics, output, &mut debugger);
            assert_eq!(State::Waiting, state.get_state(0), "f_a should be Waiting");
        }

        /*
            This test checks that a function with a Constant InputInitializer does not unblock others
            blocked sending to it when it runs. This case should not occur but it depends on the
            compiler enforcing it. Here we wish to ensure the run-time is robust to this circumstance.
        */
        #[test]
        fn constant_initializer_not_unblock() {
            let f_a = super::test_function_a_to_b();
            let f_b = Function::new("fB".to_string(), // name
                                    "/context/fB".to_string(),
                                    "/test".to_string(),
                                    vec!(Input::new(1,
                                                    &Some(Constant(ConstantInputInitializer { constant: json!(1) })),
                                                    false)),
                                    1,
                                    &vec!(), false);
            let functions = vec!(f_a, f_b);
            let mut state = RunState::new(functions, 1);
            let mut metrics = Metrics::new(2);
            let mut debugger = Some(Debugger::new(test_debug_client()));
            state.init();
            assert_eq!(State::Ready, state.get_state(1), "f_b should be Ready initially");
            assert_eq!(State::Blocked, state.get_state(0), "f_a should be in Blocked initially");

            assert_eq!(1, state.next_job().unwrap().function_id, "next() should return a job for function_id=1 (f_b) for running");

            // Event: run f_b
            let output = Output {
                job_id: 1,
                function_id: 1,
                input_values: vec!(vec!(json!(1))),
                result: (Some(json!(1)), true),
                destinations: vec!(),
                error: None,

            };
            state.process_output(&mut metrics, output, &mut debugger);

            // Test
            assert_eq!(State::Ready, state.get_state(1), "f_b should be Ready after inputs refreshed");
            assert_eq!(State::Blocked, state.get_state(0), "f_a should still be Blocked on f_b");
        }

        /*
            Check that we can accumulat ea number of inputs values on a function's input and
            then get multiple jobs for execution for the same function
        */
        #[test]
        fn can_create_multiple_jobs() {
            let f_a = Function::new("f_a".to_string(), // name
                                    "/context/fA".to_string(),
                                    "/test".to_string(),
                                    vec!(Input::new(1, &None, false)),
                                    0,
                                    &vec!(), false);
            let functions = vec!(f_a);
            let mut state = RunState::new(functions, 4);
            let mut metrics = Metrics::new(1);
            let mut debugger = Some(Debugger::new(test_debug_client()));
            state.init();

            // Send multiple inputs to f_a
            state.send_value(1, "/", 0, 0, &json!(1), &mut metrics, &mut debugger);
            state.send_value(1, "/", 0, 0, &json!(1), &mut metrics, &mut debugger);

            // Test
            assert_eq!(0, state.next_job().unwrap().function_id, "next() should return a job for function_id=0 (f_a) for running");
            assert_eq!(0, state.next_job().unwrap().function_id, "next() should return a second job for function_id=0 (f_a) for running");
        }

        /*
            Check that we can accumulate a number of inputs values on a function's input when an array
            of the same type is sent to it, and then get multiple jobs for execution
        */
        #[test]
        fn can_create_multiple_jobs_from_array() {
            let f_a = Function::new("f_a".to_string(), // name
                                    "/context/fA".to_string(),
                                    "/test".to_string(),
                                    vec!(Input::new(1, &None, false)),
                                    0,
                                    &vec!(), false);
            let functions = vec!(f_a);
            let mut state = RunState::new(functions, 4);
            let mut metrics = Metrics::new(1);
            let mut debugger = Some(Debugger::new(test_debug_client()));
            state.init();

            // Send multiple inputs to f_a via an array
            state.send_value(1, "/", 0, 0, &json!([1, 2, 3, 4]), &mut metrics, &mut debugger);

            // Test
            assert_eq!(0, state.next_job().unwrap().function_id, "next() should return a job for function_id=0 (f_a) for running");
            assert_eq!(0, state.next_job().unwrap().function_id, "next() should return a second job for function_id=0 (f_a) for running");
            assert_eq!(0, state.next_job().unwrap().function_id, "next() should return a third job for function_id=0 (f_a) for running");
            assert_eq!(0, state.next_job().unwrap().function_id, "next() should return a fourth job for function_id=0 (f_a) for running");
        }

        /*
              Check that we can accumulate a number of inputs values on a function's input when an array
              of the same type is sent to it, and then get multiple jobs for execution
        */
        #[test]
        fn can_create_multiple_jobs_with_initializer() {
            let f_a = Function::new("f_a".to_string(), // name
                                    "/context/fA".to_string(),
                                    "/test".to_string(),
                                    vec!(Input::new(1, &None, false),
                                         Input::new(1,
                                                    &Some(Constant(ConstantInputInitializer { constant: json!(1) })),
                                                    false)),
                                    0,
                                    &vec!(), false);
            let functions = vec!(f_a);
            let mut state = RunState::new(functions, 4);
            let mut metrics = Metrics::new(1);
            let mut debugger = Some(Debugger::new(test_debug_client()));
            state.init();

            // Send multiple inputs to f_a input 0 - via an array
            state.send_value(1, "/", 0, 0, &json!([1, 2, 3, 4]), &mut metrics, &mut debugger);

            // Test
            assert_eq!(0, state.next_job().unwrap().function_id, "next() should return a job for function_id=0 (f_a) for running");
            assert_eq!(0, state.next_job().unwrap().function_id, "next() should return a second job for function_id=0 (f_a) for running");
            assert_eq!(0, state.next_job().unwrap().function_id, "next() should return a third job for function_id=0 (f_a) for running");
            assert_eq!(0, state.next_job().unwrap().function_id, "next() should return a fourth job for function_id=0 (f_a) for running");
        }
    }
}
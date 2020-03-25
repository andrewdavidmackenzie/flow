use std::collections::{HashMap, HashSet};
use std::collections::VecDeque;
use std::fmt;
use std::sync::Arc;

use flow_impl::Implementation;
use log::{debug, error, trace};
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

#[derive(Debug)]
pub struct Job {
    pub job_id: usize,
    pub function_id: usize,
    pub flow_id: usize,
    pub input_set: Vec<Vec<Value>>,
    pub destinations: Vec<OutputConnection>,
    pub implementation: Arc<dyn Implementation>,
    pub result: (Option<Value>, bool),
    pub error: Option<String>, // TODO combine those two into a Result<>
}

/// blocks: (blocking_id, blocking_io_number, blocked_id, blocked_flow_id) a blocks between functions
#[derive(PartialEq, Clone, Hash, Eq)]
pub struct Block {
    pub blocking_flow_id: usize,
    pub blocking_id: usize,
    pub blocking_io_number: usize,
    pub blocked_id: usize,
    pub blocked_flow_id: usize,
}

impl Block {
    fn new(blocking_flow_id: usize, blocking_id: usize, blocking_io_number: usize, blocked_id: usize, blocked_flow_id: usize) -> Self {
        Block {
            blocking_flow_id,
            blocking_id,
            blocking_io_number,
            blocked_id,
            blocked_flow_id,
        }
    }
}

impl fmt::Debug for Block {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "#{}({}) --> #{}({}):{}", self.blocked_id, self.blocked_flow_id,
               self.blocking_id, self.blocking_flow_id, self.blocking_io_number)
    }
}

impl fmt::Display for Block {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "#{}({}) --> #{}({}):{}", self.blocked_id, self.blocked_flow_id,
               self.blocking_id, self.blocking_flow_id, self.blocking_io_number)
    }
}

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
/// at some point. So each implementation when ran returns a "run again" flag to indicate this.
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
///
/// A run-time is within it's rights to discard this function, and then potentially other functions
/// connected to it's output and other inputs until no more can be removed.
/// This run-time does not do that, in order to keep things simple and start-up time to a minimum.
/// It relies on the compiler having done that previously.
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
    /// The vector of all functions in the flow loaded from manifest
    functions: Vec<Function>,
    /// blocked: HashSet<function_id> - list of functions by id that are blocked on sending
    blocked: HashSet<usize>,
    /// blocks: Vec<(blocking_id, blocking_io_number, blocked_id, blocked_flow_id)> - a list of blocks between functions
    blocks: HashSet<Block>,
    /// ready: Vec<function_id> - a list of functions by id that are ready to run
    ready: VecDeque<usize>,
    /// running: MultiMap<function_id, job_id> - a list of functions and jobs ids that are running
    running: MultiMap<usize, usize>,
    /// number of jobs sent for execution to date
    jobs_sent: usize,
    /// limit on the number of jobs allowed to be pending to complete (i.e. running in parallel)
    max_pending_jobs: usize,
    /// Track which flow-function combinations are considered "busy" <flow_id, function_id>
    busy_flows: MultiMap<usize, usize>,
    /// Track which functions have finished and can be unblocked when flow goes not "busy"
    /// HashMap< <flow_id>, (function_id, vector of refilled io numbers of that function)>
    pending_unblocks: HashMap<usize, HashSet<usize>>,
}

impl RunState {
    pub fn new(functions: Vec<Function>, max_jobs: usize) -> Self {
        RunState {
            functions,
            blocked: HashSet::<usize>::new(),
            blocks: HashSet::<Block>::new(),
            ready: VecDeque::<usize>::new(),
            running: MultiMap::<usize, usize>::new(),
            jobs_sent: 0,
            max_pending_jobs: max_jobs,
            busy_flows: MultiMap::<usize, usize>::new(),
            pending_unblocks: HashMap::<usize, HashSet<usize>>::new(),
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
        self.busy_flows.clear();
        self.pending_unblocks.clear();
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

        let mut inputs_ready_list = Vec::<(usize, usize)>::new();

        for function in &mut self.functions {
            debug!("Init:\tInitializing Function #{} '{}' in Flow #{}",
                   function.id(), function.name(), function.get_flow_id());
            function.init_inputs(true);
            if function.inputs_full() {
                inputs_ready_list.push((function.id(), function.get_flow_id()));
            }
        }

        // Due to initialization of some inputs other functions attempting to send to it should block
        self.create_init_blocks();

        // Put all functions that have their inputs ready and are not blocked on the `ready` list
        debug!("Init:\tReadying initial functions: inputs full and not blocked on output");
        for (id, flow_id) in inputs_ready_list {
            self.inputs_now_full(id, flow_id);
        }

        trace!("Init: State - {}", self)
    }

    /*
        Scan thru all functions and output routes for each, if the destination input is already
        full due to the init process, then create a block for the sender and added sender to blocked
        list.
    */
    fn create_init_blocks(&mut self) {
        let mut blocks = HashSet::<Block>::new();
        let mut blocked = HashSet::<usize>::new();

        debug!("Init:\tCreating any initial block entries that are needed");

        for source_function in &self.functions {
            let source_id;
            let source_flow_id;
            let destinations;
            let source_has_inputs_full;
            {
                source_id = source_function.id();
                source_flow_id = source_function.get_flow_id();
                source_has_inputs_full = source_function.inputs_full();
                destinations = source_function.output_destinations().clone();
            }

            for destination in destinations {
                if destination.function_id != source_id { // don't block yourself!
                    let destination_function = self.get(destination.function_id);
                    if destination_function.input_full(destination.io_number) {
                        trace!("Init:\t\tAdded block #{} --> #{}:{}", source_id, destination.function_id, destination.io_number);
                        blocks.insert(Block::new(destination.flow_id, destination.function_id, destination.io_number,
                                                 source_id, source_flow_id));
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

    /*
        Figure out the state of a function based on it's preence or not in the different control
        lists
    */
    pub fn get_state(&self, function_id: usize) -> State {
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

        for block in &self.blocks {
            if block.blocked_flow_id == function_id {
                output.push_str(&format!("\t{:?}\n", block));
            } else if block.blocking_id == function_id {
                output.push_str(&format!("\tBlocking #{}:{} <-- Blocked #{}\n",
                                         block.blocking_id, block.blocking_io_number, block.blocked_id));
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
        if self.ready.is_empty() || self.number_jobs_running() >= self.max_pending_jobs {
            return None;
        }

        // create a job for the function_id at the head of the ready list
        match self.ready.remove(0) {
            Some(function_id) => {
                let job = self.create_job(function_id);

                // unblock senders blocked trying to send to this function's empty inputs
                self.unblock_senders(job.job_id, job.function_id, job.flow_id);

                Some(job)
            }
            None => None
        }
    }

    /*
        Track the number of jobs sent to date
    */
    pub fn job_sent(&mut self, job_id: usize) {
        self.jobs_sent += 1;
        trace!("Job #{}:\tSent - {}", job_id, self);
    }

    /*
        return the number of jobs sent to date
    */
    pub fn jobs_sent(&self) -> usize {
        self.jobs_sent
    }

    /*
        Given a function id, prepare a job for execution that contains the input values, the
        implementation and the destination functions the output should be sent to when done
    */
    fn create_job(&mut self, function_id: usize) -> Job {
        let job_id = self.jobs_sent;

        let function = self.get_mut(function_id);

        let input_set = function.take_input_set();
        let flow_id = function.get_flow_id();

        debug!("Job #{}:\tCreating for Function #{} '{}' ---------------------------", job_id, function_id, function.name());

        // inputs were taken and hence emptied - so refresh any inputs that have constant initializers for next time
        function.init_inputs(false);

        debug!("Job #{}:\tInputs: {:?}", job_id, input_set);

        let implementation = function.get_implementation();

        let destinations = function.output_destinations().clone();

        Job {
            job_id,
            function_id,
            flow_id,
            implementation,
            input_set,
            destinations,
            result: (None, false),
            error: None,
        }
    }

    /*
        Complete a Job by takingits output and updating the runlist accordingly.

        If other functions were blocked trying to send to this one - we can now unblock them
        as it has consumed it's inputs and they are free to be sent to again.

        Then take the output and send it to all destination IOs on different function it should be
        sent to, marking the source function as blocked because those others must consume the output
        if those other function have all their inputs, then mark them accordingly.
    */
    pub fn complete_job(&mut self, metrics: &mut Metrics, job: Job, debugger: &mut Option<Debugger>) {
        trace!("Job #{}:\tCompleted by Function #{}", job.job_id, job.function_id);
        self.running.retain(|&_, &job_id| job_id != job.job_id);
        let job_id = job.job_id;

        match job.error {
            None => {
                let output_value = job.result.0;
                let function_can_run_again = job.result.1;

                // if it produced an output value
                if let Some(output_v) = output_value {
                    debug!("Job #{}:\tOutputs '{}'", job.job_id, output_v);

                    for destination in &job.destinations {
                        match output_v.pointer(&destination.subpath) {
                            Some(output_value) =>
                                self.send_value(job.function_id,
                                                job.flow_id,
                                                &destination.subpath,
                                                destination.function_id,
                                                destination.io_number, output_value, metrics, debugger),
                            _ => trace!("Job #{}:\t\tNo output value found at '{}'", job.job_id, &destination.subpath)
                        }
                    }
                }

                self.remove_from_busy(job.function_id);

                // if it wants to run again, it can p then add back to the Ready list
                if function_can_run_again {
                    self.refill_inputs(job.function_id, job.flow_id);
                }

                // need to do flow unblocks as that could affect other functions even if this one cannot run again
                self.unblock_flows(job.flow_id, job.job_id);
            }
            Some(_) => {
                match debugger {
                    None => error!("Job #{}:\tError in Job execution:\n{:#?}", job.job_id, job),
                    Some(debugger) => debugger.panic(&self, job)
                }
            }
        }

        #[cfg(feature = "checks")]
            self.check_invariants(job_id);
    }

    /*
        Send a value produced as part of an output of running a job to a destination function on
        a specific input, update the metrics and potentially enter the debugger
    */
    fn send_value(&mut self, source_id: usize, source_flow_id: usize, output_route: &str, destination_id: usize, io_number: usize,
                  output_value: &Value, metrics: &mut Metrics, debugger: &mut Option<Debugger>) {
        if output_route.is_empty() {
            debug!("\t\tFunction #{} sending '{}' to Function #{}:{}",
                   source_id, output_value, destination_id, io_number);
        } else {
            debug!("\t\tFunction #{} sending '{}' via output route '{}' to Function #{}:{}",
                   source_id, output_value, output_route, destination_id, io_number);
        }

        if let Some(ref mut debugger) = debugger {
            debugger.check_prior_to_send(self, source_id, output_route,
                                         &output_value, destination_id, io_number);
        }

        let destination = self.get_mut(destination_id);
        let destination_flow_id = destination.get_flow_id();
        destination.write_input(io_number, output_value);

        #[cfg(feature = "metrics")]
            metrics.increment_outputs_sent();

        // for the case when a function is sending to itself:
        // - avoid blocking on itself
        // - delay determining if it should be in the blocked or ready lists (by calling inputs_now_full())
        //   until it has sent all it's other outputs as it might be blocked by another function.
        let block = destination.input_full(io_number) && (source_id != destination_id);
        let full = destination.inputs_full() && (source_id != destination_id);

        if block {
            self.create_block(destination_flow_id, destination_id, io_number, source_id, source_flow_id, debugger);
        }

        if full {
            self.inputs_now_full(destination_id, destination_flow_id);
        }
    }

    /*
        Refresh any inputs that have initializers on them, and return true if there are now enough
        input values to create a job for the function.
    */
    fn refill_inputs(&mut self, function_id: usize, flow_id: usize) {
        // TODO see if we can find a way to avoid accessing the function here, just update the
        // ready status of the id of the function and pickup the inputs when we create the job
        let function = self.get_mut(function_id);

        function.init_inputs(false);

        if function.inputs_full() {
            self.inputs_now_full(function_id, flow_id);
        }
    }

    pub fn start(&mut self, job: &Job) {
        self.running.insert(job.function_id, job.job_id);
    }

    #[cfg(feature = "debugger")]
    pub fn get_output_blockers(&self, id: usize) -> Vec<(usize, usize)> {
        let mut blockers = vec!();

        for block in &self.blocks {
            if block.blocked_id == id {
                blockers.push((block.blocking_id, block.blocking_io_number));
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
    fn inputs_now_full(&mut self, id: usize, flow_id: usize) {
        if self.blocked_sending(id) {
            // It has inputs and could run, if it weren't blocked on output
            debug!("\t\t\t\tFunction #{}, inputs full, but blocked on output. Added to blocked list", id);
            // so put it on the blocked list
            self.blocked.insert(id);
        } else {
            // It has inputs, and is not blocked on output, so it can run! Mark as ready to run.
            debug!("\t\t\t\tFunction #{} not blocked on output, so added to 'Ready' list", id);
            self.mark_ready(id, flow_id);
        }
    }

    /*
        Mark a function "ready" to run, by adding it's id to the ready list
    */
    fn mark_ready(&mut self, function_id: usize, flow_id: usize) {
        self.ready.push_back(function_id);
        self.busy_flows.insert(flow_id, function_id);
    }

    // See if there is any block where the blocked function is the one we're looking for
    fn blocked_sending(&self, id: usize) -> bool {
        for block in &self.blocks {
            if block.blocked_id == id {
                return true;
            }
        }
        false
    }

    pub fn jobs(&self) -> usize {
        self.jobs_sent
    }

    pub fn num_functions(&self) -> usize {
        self.functions.len()
    }

    /*
        The function blocker_function_id in flow blocked_flow_id has completed execution and so
        is a candidate to send to from other functions that were blocked sending to it previously.

        But we don't want to unblock them to send to it, until all other functions inside this flow
        are idle, and hence the flow becomes idle.
    */
    pub fn unblock_senders(&mut self, job_id: usize, blocker_function_id: usize, blocker_flow_id: usize) {
        // delete blocks to this function from within the same flow
        let flow_internal_blocks = |block: &Block| block.blocking_flow_id == block.blocked_flow_id;

        self.unblock_senders_to_function(blocker_function_id, flow_internal_blocks);

        // Add this function to the pending unblock list for later when flow goes idle - ensure entry is unique
        let mut new_set = HashSet::new();
        new_set.insert(blocker_function_id);
        let set = self.pending_unblocks.entry(blocker_flow_id).or_insert(new_set);
        set.insert(blocker_function_id);
        trace!("Job #{}:\t\tAdded a pending_unblock --> #{}({})", job_id, blocker_function_id, blocker_flow_id);
    }

    /*
        Detect which flows have gone inactive and remove pending unblocks for functions in it
    */
    fn unblock_flows(&mut self, blocker_flow_id: usize, job_id: usize) {
        let any_block = |_block: &Block| true;

        // if flow is now idle, remove any blocks on sending to functions in the flow
        if self.busy_flows.get(&blocker_flow_id).is_none() {
            trace!("Job #{}:\tFlow #{} is now idle, so removing pending_unblocks for flow #{}",
                   job_id, blocker_flow_id, blocker_flow_id);

            if let Some(unblocks) = self.pending_unblocks.remove(&blocker_flow_id) {
                trace!("Job #{}:\tRemoving pending unblocks to functions in Flow #{}", job_id, blocker_flow_id);
                for unblock_function_id in unblocks {
                    self.unblock_senders_to_function(unblock_function_id, any_block);
                }
            }
        }
    }

    /*
        Remove ONE entry of <flow_id, function_id> from the busy_flows multimap
    */
    fn remove_from_busy(&mut self, blocker_function_id: usize) {
        // Remove this flow-function combination from the busy flow list - if it's not also ready for other jobs
        if !self.ready.contains(&blocker_function_id) {
            let mut count = 0;
            self.busy_flows.retain(|&_flow_id, &function_id| {
                if function_id == blocker_function_id && count == 0 {
                    count += 1;
                    false // remove it
                } else {
                    true // retain it
                }
            });
            trace!("\t\t\tUpdated busy_flows list to: {:?}", self.busy_flows);
        }
    }

    /*
        unblock all functions that were blocked trying to send to blocker_function_id by removing all entries
        in the `blocks` list where the first value (blocking_id) matches blocker_function_id.

        Once each is unblocked, if it's inputs are full, then it is ready to be run again,
        so mark as ready
    */
    fn unblock_senders_to_function<F>(&mut self, blocker_function_id: usize, f: F) where F: Fn(&Block) -> bool {
        let mut unblock_list = vec!();

        // Avoid unblocking multiple functions blocked on sending to the same input, just unblock the first
        let mut unblock_io_numbers = vec!();

        // don't unblock more than one function sending to each io port
        // don't unblock functions sending to an io port that was previously refilled
        trace!("\t\t\tRemoving blocks to Function #{}", blocker_function_id);
        self.blocks.retain(|block| {
            if (block.blocking_id == blocker_function_id) &&
                !unblock_io_numbers.contains(&block.blocking_io_number) &&
                f(block)
            {
                unblock_list.push((block.blocked_id, block.blocked_flow_id));
                unblock_io_numbers.push(block.blocking_io_number);
                trace!("\t\t\tBlock removed {:?}", block);
                false // remove this block
            } else {
                true // retain this block
            }
        });

        // update the state of the functions that have been unblocked
        // Note: they could be blocked on other functions apart from the the one that just unblocked
        for (unblocked_id, unblocked_flow_id) in unblock_list {
            if self.blocked.contains(&unblocked_id) && !self.blocked_sending(unblocked_id) {
                debug!("\t\t\t\tFunction #{} removed from 'blocked' list", unblocked_id);
                self.blocked.remove(&unblocked_id);

                if self.get(unblocked_id).inputs_full() {
                    debug!("\t\t\t\tFunction #{} has inputs ready, so added to 'ready' list", unblocked_id);
                    self.mark_ready(unblocked_id, unblocked_flow_id);
                }
            }
        }
    }

    /*
        Create a 'block" indicating that function 'blocked_id' cannot run as it has an output
        destination to an input on function 'blocking_id' that is already full.
    */
    fn create_block(&mut self, blocking_flow_id: usize, blocking_id: usize, blocking_io_number: usize,
                    blocked_id: usize, blocked_flow_id: usize, debugger: &mut Option<Debugger>) {
        let block = Block::new(blocking_flow_id, blocking_id, blocking_io_number, blocked_id, blocked_flow_id);
        trace!("\t\t\t\t\tCreating Block {:?}", block);

        if !self.blocks.contains(&block) {
            self.blocks.insert(block.clone());
            if let Some(ref mut debugger) = debugger {
                debugger.check_on_block_creation(self, &block);
            }
        }
    }

    #[cfg(feature = "checks")]
    fn runtime_error(&self, job_id: usize, message: &str, file: &str, line: u32) {
        error!("Job #{}: Runtime error: at file: {}, line: {}\n\t\t{}", job_id, file, line, message);
        error!("Job #{}: Error State - {}", job_id, self);
        panic!();
    }

    /*
        Check a number of "invariants" i.e. unbreakable rules about the state, and go into debugger
        if one is found to be broken, with a message explaining it
    */
    #[cfg(feature = "checks")]
    fn check_invariants(&mut self, job_id: usize) {
        // check invariants of each functions
        for function in &self.functions {
            match self.get_state(function.id()) {
                State::Ready => {
                    if !self.busy_flows.contains_key(&function.get_flow_id()) {
                        return self.runtime_error(job_id, &format!("Function #{} is Ready, but Flow #{} is not busy", function.id(), function.get_flow_id()),
                                                  file!(), line!());
                    }
                }
                State::Running => {
                    if !self.busy_flows.contains_key(&function.get_flow_id()) {
                        return self.runtime_error(job_id, &format!("Function #{} is Running, but Flow #{} is not busy", function.id(), function.get_flow_id()),
                                                  file!(), line!());
                    }
                }
                State::Blocked => {
                    if !self.blocked_sending(function.id()) {
                        return self.runtime_error(job_id, &format!("Function #{} is in Blocked state, but no block exists", function.id()),
                                                  file!(), line!());
                    }
                }
                State::Waiting => {}
            }

            // State::Running is because functions with initializers auto-refill when sent to run
            // So they will show as inputs full, but not Ready or Blocked
            let state = self.get_state(function.id());
            if (function.inputs().len() > 0) && function.inputs_full() &&
                !(state == State::Ready || state == State::Blocked || state == State::Running) {
                error!("{}", function);
                return self.runtime_error(job_id, &format!("Function #{} inputs are full, but it is not Ready or Blocked", function.id()),
                                          file!(), line!());
            }
        }

        // Check block invariants
        for block in &self.blocks {
            // function should not be blocked on itself
            if block.blocked_id == block.blocking_id {
                return self.runtime_error(job_id, &format!("Block {} has same Function id as blocked and blocking", block),
                                          file!(), line!());
            }

            // For each block on a destination function, then either that input should be full or
            // the function should be running in parallel with the one that just completed
            // or it's flow should be busy and there should be a pending unblock on it
            if !(self.functions.get(block.blocking_id).unwrap().input_full(block.blocking_io_number) ||
                (self.busy_flows.contains_key(&block.blocking_flow_id) && self.pending_unblocks.contains_key(&block.blocking_flow_id))) {
                return self.runtime_error(job_id,
                                          &format!("Block {} exists for function #{}, but Function #{}:{} input is not full",
                                                   block, block.blocking_id, block.blocking_id, block.blocking_io_number),
                                          file!(), line!());
            }
        }

        // Check pending unblock invariants
        for pending_unblock_flow_id in self.pending_unblocks.keys() {
            // flow it's in must be busy
            if !self.busy_flows.contains_key(pending_unblock_flow_id) {
                return self.runtime_error(job_id, &format!("Pending Unblock exists for Flow #{}, but it is not busy", pending_unblock_flow_id),
                                          file!(), line!());
            }
        }

        // Check busy flow invariants
        for (flow_id, function_id) in self.busy_flows.iter() {
            let state = self.get_state(*function_id);
            if !(state == State::Ready || state == State::Running) {
                return self.runtime_error(job_id, &format!("Busy flow entry exists for Function #{} in Flow #{} but it's state is {:?}",
                                                           function_id, flow_id, state),
                                          file!(), line!());
            }
        }
    }
}

#[cfg(any(feature = "logging", feature = "debugger"))]
impl fmt::Display for RunState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "RunState:\n")?;
        write!(f, "    Jobs Executed: {}\n", self.jobs_sent)?;
        write!(f, "Functions Blocked: {:?}\n", self.blocked)?;
        write!(f, "           Blocks: {:?}\n", self.blocks)?;
        write!(f, "  Functions Ready: {:?}\n", self.ready)?;
        write!(f, "Functions Running: {:?}\n", self.running)?;
        write!(f, "       Flows Busy: {:?}\n", self.busy_flows)?;
        write!(f, " Pending Unblocks: {:?}", self.pending_unblocks)
    }
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use flow_impl::Implementation;
    use serde_json::json;
    use serde_json::Value;

    use crate::debug_client::{DebugClient, Event, Response};
    use crate::debug_client::{Command, Param};
    use crate::function::Function;
    use crate::input::Input;
    use crate::input::InputInitializer::OneTime;
    use crate::input::OneTimeInputInitializer;
    use crate::output_connection::OutputConnection;
    use crate::run_state;

    use super::Job;

    #[derive(Debug)]
    struct TestImpl {}

    impl Implementation for TestImpl {
        fn run(&self, _inputs: Vec<Vec<Value>>) -> (Option<Value>, bool) {
            unimplemented!()
        }
    }

    fn test_impl() -> Arc<dyn Implementation> {
        Arc::new(TestImpl {})
    }

    // Helpers
    struct TestDebugClient {}

    impl DebugClient for TestDebugClient {
        fn init(&self) {}

        fn get_command(&self, _job_number: usize) -> Command {
            Command::Step(Some(run_state::test::Param::Numeric(1)))
        }

        fn send_event(&self, _event: Event) {}

        fn send_response(&self, _response: Response) {}
    }

    fn test_debug_client() -> &'static dyn DebugClient {
        &TestDebugClient {}
    }

    fn test_function_a_to_b_not_init() -> Function {
        let connection_to_f1 = OutputConnection::new("".to_string(),
                                                     1, 0, 0,
                                                     Some("/fB".to_string()));

        Function::new("fA".to_string(), // name
                      "/fA".to_string(),
                      "/test".to_string(),
                      vec!(Input::new(1, &None, false)),
                      0, 0,
                      &vec!(connection_to_f1), false) // outputs to fB:0
    }

    fn test_function_a_to_b() -> Function {
        let connection_to_f1 = OutputConnection::new("".to_string(),
                                                     1, 0, 0,
                                                     Some("/fB".to_string()));
        Function::new("fA".to_string(), // name
                      "/fA".to_string(),
                      "/test".to_string(),
                      vec!(Input::new(1,
                                      &Some(OneTime(OneTimeInputInitializer { once: json!(1) })),
                                      false)),
                      0, 0,
                      &vec!(connection_to_f1), false) // outputs to fB:0
    }

    fn test_function_a_init() -> Function {
        Function::new("fA".to_string(), // name
                      "/fA".to_string(),
                      "/test".to_string(),
                      vec!(Input::new(1,
                                      &Some(OneTime(OneTimeInputInitializer { once: json!(1) })),
                                      false)),
                      0, 0,
                      &vec!(), false)
    }

    fn test_function_b_not_init() -> Function {
        Function::new("fB".to_string(), // name
                      "/fB".to_string(),
                      "/test".to_string(),
                      vec!(Input::new(1, &None, false)),
                      1, 0,
                      &vec!(), false)
    }

    fn test_function_b_init() -> Function {
        Function::new("fB".to_string(), // name
                      "/fB".to_string(),
                      "/test".to_string(),
                      vec!(Input::new(1,
                                      &Some(OneTime(OneTimeInputInitializer { once: json!(1) })),
                                      false)),
                      1, 0,
                      &vec!(), false)
    }

    fn test_output(source_function_id: usize, dest_function_id: usize) -> Job {
        let out_conn = OutputConnection::new("".to_string(), dest_function_id, 0, 0, None);
        Job {
            job_id: 1,
            function_id: source_function_id,
            flow_id: 0,
            implementation: test_impl(),
            input_set: vec!(vec!(json!(1))),
            result: (Some(json!(1)), true),
            destinations: vec!(out_conn),
            error: None,
        }
    }

    fn error_output(source_function_id: usize, dest_function_id: usize) -> Job {
        let out_conn = OutputConnection::new("".to_string(), dest_function_id, 0, 0, None);
        Job {
            job_id: 1,
            flow_id: 0,
            implementation: test_impl(),
            function_id: source_function_id,
            input_set: vec!(vec!(json!(1))),
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
            state.job_sent(0);
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

        use super::super::Job;
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
                                    "/fA".to_string(),
                                    "/test".to_string(),
                                    vec!(Input::new(1, &None, false)),
                                    0, 0,
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
            let f_a = Function::new("fA".to_string(),
                                    "/fA".to_string(),
                                    "/test".to_string(),
                                    vec!(Input::new(1, &None, false)),
                                    0, 0,
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
            let functions = vec!(f_a, f_b);
            let mut state = RunState::new(functions, 1);
            let mut metrics = Metrics::new(2);
            let mut debugger = Some(Debugger::new(test_debug_client()));

            // Initial state
            state.init();
            assert_eq!(State::Ready, state.get_state(1), "f_b should be Ready");
            assert_eq!(State::Blocked, state.get_state(0), "f_a should be in Blocked state, by f_b");

            // First job
            let job = state.next_job().unwrap();
            assert_eq!(1, job.function_id, "next() should return function_id=1 (f_b) for running");
            state.start(&job);
            assert_eq!(State::Running, state.get_state(1), "f_b should be Running");

// Event
            let output = super::test_output(1, 0);
            state.complete_job(&mut metrics, output, &mut debugger);

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
            state.complete_job(&mut metrics, output, &mut debugger);

            assert_eq!(State::Waiting, state.get_state(1), "f_b should be Waiting");
        }

        #[test]
        fn running_to_ready_on_done() {
            let f_a = Function::new("fA".to_string(), // name
                                    "/fA".to_string(),
                                    "/test".to_string(),
                                    vec!(Input::new(1,
                                                    &Some(Constant(ConstantInputInitializer { constant: json!(1) })),
                                                    false)),
                                    0, 0,
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
            let job = Job {
                job_id: 1,
                function_id: 0,
                flow_id: 0,
                implementation: super::test_impl(),
                input_set: vec!(vec!(json!(1))),
                result: (None, true),
                destinations: vec!(),
                error: None,
            };
            state.complete_job(&mut metrics, job, &mut debugger);

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
            let job = Job {
                job_id: 0,
                function_id: 0,
                flow_id: 0,
                implementation: super::test_impl(),
                input_set: vec!(vec!(json!(1))),
                result: (None, true),
                destinations: vec!(),
                error: None,
            };
            state.complete_job(&mut metrics, job, &mut debugger);

// Test
            assert_eq!(State::Waiting, state.get_state(0), "f_a should be Waiting again");
        }

        // Done: at least one destination input is full, so can't run  running_to_blocked_on_done
        #[test]
        fn running_to_blocked_on_done() {
            let out_conn = OutputConnection::new("".to_string(), 1, 0, 0, None);
            let f_a = Function::new("fA".to_string(), // name
                                    "/fA".to_string(),
                                    "/test".to_string(),
                                    vec!(Input::new(1,
                                                    &Some(Constant(ConstantInputInitializer { constant: json!(1) })),
                                                    false)),
                                    0, 0,
                                    &vec!(out_conn), false); // outputs to fB:0
            let f_b = Function::new("fB".to_string(), // name
                                    "/fB".to_string(),
                                    "/test".to_string(),
                                    vec!(Input::new(1, &None, false)),
                                    1, 0,
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
            state.complete_job(&mut metrics, output, &mut debugger);

// Test f_a should transition to Blocked on f_b
            assert_eq!(State::Blocked, state.get_state(0), "f_a should be Blocked");
        }

        #[test]
        fn waiting_to_ready_on_input() {
            let f_a = Function::new("fA".to_string(), // name
                                    "/fA".to_string(),
                                    "/test".to_string(),
                                    vec!(Input::new(1, &None, false)),
                                    0, 0,
                                    &vec!(), false);
            let out_conn = OutputConnection::new("".into(), 0, 0, 0, None);
            let f_b = Function::new("fB".to_string(), // name
                                    "/fB".to_string(),
                                    "/test".to_string(),
                                    vec!(Input::new(1, &None, false)),
                                    1, 0,
                                    &vec!(out_conn), false);
            let functions = vec!(f_a, f_b);
            let mut state = RunState::new(functions, 1);
            let mut metrics = Metrics::new(1);
            let mut debugger = Some(Debugger::new(test_debug_client()));
            state.init();
            assert_eq!(State::Waiting, state.get_state(0), "f_a should be Waiting");

// Event run f_b which will send to f_a
            let output = super::test_output(1, 0);
            state.complete_job(&mut metrics, output, &mut debugger);

// Test
            assert_eq!(State::Ready, state.get_state(0), "f_a should be Ready");
        }

        /*
            fA (#0) has an input but not initialized, outputs to #1 (fB)
            fB (#1) has an input with a ConstantInitializer, outputs back to #0 (fA)
        */
        #[test]
        fn waiting_to_blocked_on_input() {
            let f_a = super::test_function_a_to_b_not_init();
            let connection_to_f0 = OutputConnection::new("".into(), 0, 0, 0, None);
            let f_b = Function::new("fB".to_string(), // name
                                    "/fB".to_string(),
                                    "/test".to_string(),
                                    vec!(Input::new(1,
                                                    &Some(Constant(ConstantInputInitializer { constant: json!(1) })),
                                                    false)),
                                    1, 0,
                                    &vec!(connection_to_f0), false);
            let functions = vec!(f_a, f_b);
            let mut state = RunState::new(functions, 1);
            let mut metrics = Metrics::new(1);
            let mut debugger = Some(Debugger::new(test_debug_client()));
            state.init();

            assert_eq!(state.get_state(1), State::Ready, "f_b should be Ready");
            assert_eq!(state.get_state(0), State::Waiting, "f_a should be in Waiting");

            assert_eq!(state.next_job().unwrap().function_id, 1, "next() should return function_id=1 (f_b) for running");

            // create output from f_b as if it had run - will send to f_a
            let output = super::test_output(1, 0);
            state.complete_job(&mut metrics, output, &mut debugger);

            // Test
            assert_eq!(state.get_state(0), State::Ready, "f_a should be Ready");
        }

        /*
            This tests that if a function that has a loop back sending to itself, runs the first time
            due to a OnceInitializer, that after running it sends output back to itself and is ready
            (not waiting for an input from elsewhere and no deadlock due to blocking itself occurs
        */
        #[test]
        fn not_block_on_self() {
            let connection_to_0 = OutputConnection::new("".to_string(), 0, 0, 0, None);
            let connection_to_1 = OutputConnection::new("".to_string(), 1, 0, 0, None);

            let f_a = Function::new("fA".to_string(), // name
                                    "/fA".to_string(),
                                    "/test".to_string(),
                                    vec!(Input::new(1,
                                                    &Some(OneTime(OneTimeInputInitializer { once: json!(1) })),
                                                    false)),
                                    0, 0,
                                    &vec!(
                                        connection_to_0.clone(), // outputs to self:0
                                        connection_to_1.clone() // outputs to f_b:0
                                    ), false);
            let f_b = Function::new("fB".to_string(), // name
                                    "/fB".to_string(),
                                    "/test".to_string(),
                                    vec!(Input::new(1, &None, false)),
                                    1, 0,
                                    &vec!(), false);
            let functions = vec!(f_a, f_b); // NOTE the order!
            let mut state = RunState::new(functions, 1);
            let mut metrics = Metrics::new(2);
            let mut debugger = Some(Debugger::new(test_debug_client()));
            state.init();

            assert_eq!(state.get_state(0), State::Ready, "f_a should be Ready");
            assert_eq!(state.get_state(1), State::Waiting, "f_b should be in Waiting");

            assert_eq!(state.next_job().unwrap().function_id, 0, "next() should return function_id=0 (f_a) for running");

            // Event: run f_a
            let job = Job {
                job_id: 0,
                function_id: 0,
                flow_id: 0,
                implementation: super::test_impl(),
                input_set: vec!(vec!(json!(1))),
                result: (Some(json!(1)), true),
                destinations: vec!(connection_to_0, connection_to_1),
                error: None,

            };
            state.complete_job(&mut metrics, job, &mut debugger);

            // Test
            assert_eq!(state.get_state(1), State::Ready, "f_b should be Ready");
            assert_eq!(state.get_state(0), State::Blocked, "f_a should be Blocked on f_b");

            let job = state.next_job().unwrap();
            assert_eq!(job.function_id, 1, "next() should return function_id=1 (f_b) for running");
            state.complete_job(&mut metrics, job, &mut debugger);

            let job = state.next_job().unwrap();
            assert_eq!(job.function_id, 0, "next() should return function_id=0 (f_a) for running");
            state.complete_job(&mut metrics, job, &mut debugger);
        }
    }

    /****************************** Miscelaneous tests **************************/
    mod functional_tests {
        use serde_json::json;

        use crate::debugger::Debugger;
        use crate::function::Function;
        use crate::input::Input;
        use crate::metrics::Metrics;
        use crate::output_connection::OutputConnection;

        use super::super::Job;
        use super::super::RunState;
        use super::super::State;
        use super::test_debug_client;

        fn test_functions<'a>() -> Vec<Function> {
            let out_conn1 = OutputConnection::new("".to_string(), 1, 0, 0, None);
            let out_conn2 = OutputConnection::new("".to_string(), 2, 0, 0, None);
            let p0 = Function::new("p0".to_string(), // name
                                   "/p0".to_string(),
                                   "/test".to_string(),
                                   vec!(), // input array
                                   0, 0,
                                   &vec!(out_conn1, out_conn2) // destinations
                                   , false);    // implementation
            let p1 = Function::new("p1".to_string(),
                                   "/p1".to_string(),
                                   "/test".to_string(),
                                   vec!(Input::new(1, &None, false)), // inputs array
                                   1, 0,
                                   &vec!(), false);
            let p2 = Function::new("p2".to_string(),
                                   "/p2".to_string(),
                                   "/test".to_string(),
                                   vec!(Input::new(1, &None, false)), // inputs array
                                   2, 0,
                                   &vec!(), false);
            vec!(p0, p1, p2)
        }

        #[test]
        fn blocked_works() {
            let mut state = RunState::new(test_functions(), 1);
            let mut debugger = Some(Debugger::new(test_debug_client()));

// Indicate that 0 is blocked by 1 on input 0
            state.create_block(0, 1, 0, 0, 0, &mut debugger);
            assert!(state.blocked_sending(0));
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
            state.inputs_now_full(0, 0);

            assert_eq!(state.next_job().unwrap().function_id, 0);
        }

        #[test]
        fn inputs_ready_makes_ready() {
            let mut state = RunState::new(test_functions(), 1);

// Put 0 on the blocked/ready list depending on blocked status
            state.inputs_now_full(0, 0);

            assert_eq!(state.next_job().unwrap().function_id, 0);
        }

        #[test]
        fn blocked_is_not_ready() {
            let mut state = RunState::new(test_functions(), 1);
            let mut debugger = Some(Debugger::new(test_debug_client()));

// Indicate that 0 is blocked by 1 on input 0
            state.create_block(0, 1, 0, 0, 0, &mut debugger);

// Put 0 on the blocked/ready list depending on blocked status
            state.inputs_now_full(0, 0);

            assert!(state.next_job().is_none());
        }

        #[test]
        fn unblocking_makes_ready() {
            let mut state = RunState::new(test_functions(), 1);
            let mut debugger = Some(Debugger::new(test_debug_client()));

            // Indicate that 0 is blocked by 1 and put 0 on the blocked list
            state.create_block(0, 1, 0, 0, 0, &mut debugger);
            // 0's inputs are now full, so it would be ready if it weren't blocked on output
            state.inputs_now_full(0, 0);
            // 0 does not show as ready.
            assert!(state.next_job().is_none());

            // now unblock senders to 1 (i.e. 0)
            state.unblock_senders(0, 1, 0);

            // Now function with id 0 should be ready and served up by next
            assert_eq!(state.next_job().unwrap().function_id, 0);
        }

        #[test]
        fn unblocking_doubly_blocked_functions_not_ready() {
            let mut state = RunState::new(test_functions(), 1);
            let mut debugger = Some(Debugger::new(test_debug_client()));

// Indicate that 0 is blocked by 1 and 2
            state.create_block(0, 1, 0, 0, 0, &mut debugger);
            state.create_block(0, 2, 0, 0, 0, &mut debugger);

// Put 0 on the blocked/ready list depending on blocked status
            state.inputs_now_full(0, 0);

            assert!(state.next_job().is_none());

// now unblock 0 by 1
            state.unblock_senders(0, 1, 0);

// Now function with id 0 should still not be ready as still blocked on 2
            assert!(state.next_job().is_none());
        }

        #[test]
        fn wont_return_too_many_jobs() {
            let mut state = RunState::new(test_functions(), 1);

// Put 0 on the ready list
            state.inputs_now_full(0, 0);
// Put 1 on the ready list
            state.inputs_now_full(1, 0);

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
            let job = Job {
                job_id: 0,
                function_id: 0,
                flow_id: 0,
                implementation: super::test_impl(),
                input_set: vec!(vec!(json!(1))),
                result: (Some(json!(1)), true),
                destinations: vec!(),
                error: None,
            };

// Test there is no problem producing an Output when no destinations to send it to
            state.complete_job(&mut metrics, job, &mut debugger);
            assert_eq!(State::Waiting, state.get_state(0), "f_a should be Waiting");
        }
    }
}
use std::collections::{HashMap, HashSet};
use std::collections::VecDeque;
use std::fmt;
use std::time::Duration;

use log::{debug, error, trace};
use multimap::MultiMap;
use serde_derive::{Deserialize, Serialize};
use serde_json::{json, Value};

#[cfg(feature = "metrics")]
use flowcore::model::metrics::Metrics;
use flowcore::model::output_connection::OutputConnection;
use flowcore::model::output_connection::Source::{Input, Output};
use flowcore::model::runtime_function::RuntimeFunction;
use flowcore::model::submission::Submission;

use crate::block::Block;
#[cfg(feature = "debugger")]
use crate::debugger::Debugger;
use crate::job::Job;

/// `State` represents the possible states it is possible for a function to be in
#[cfg(any(debug_assertions, feature = "debugger", test))]
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum State {
    /// Ready     - Function will be in Ready state when all of it's inputs are full and there are no inputs
    ///           it sends to that are full (unless that input is it's own)
    Ready,
    /// Blocked   - Function is in Blocked state when there is at least one input it sends to that is full
    ///           (unless that input is it's own, as then it will be emptied when the function runs)
    Blocked,
    /// Waiting   - Function is in Blocked state when at least one of it's inputs is not full
    Waiting,
    /// Running   - Function is in Running state when it has been picked from the Ready list for execution
    ///           using the next() function
    Running,
    /// Completed - Function has indicated that it no longer wants to be run, so it's execution
    ///           has completed.
    Completed,
}

/// `RunState` is a structure that maintains the state of all the functions in the currently
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
/// A function may need to only run once, or to stop being executed repeatedly at some point. 
/// So each implementation when ran returns a "run again" flag to indicate this.
/// An example of functions that may decide to stop running are:
/// - args: produces arguments from the command line execution of a flow once at start-up
/// - readline: read a line of input from standard input, until End-of-file (EOF) is detected.
///   If this was not done, then the flow would never stop running as the readline function would
///   always be re-run and waiting for more input, but none would ever be received after EOF.
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
/// Initializers
/// ============
/// There are two types of initializers on inputs:
/// * "Once" - the input is filled with the specified value once at start-up.
/// * "Constant" - after the functions runs the input refilled with the same value.
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
/// Init    Ready     No inputs and no destination input full                     to_ready_1_on_init
///                   All inputs initialized and no destination input full        to_ready_2_on_init
///                   All inputs initialized and no destinations                  to_ready_3_on_init
/// Init    Blocked   Some destination input is full                              to_blocked_on_init
/// Init    Waiting   At least one input is not full                              to_waiting_on_init
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
/// Iteration and Recursion
/// =======================
/// A function may send values to itself using a loop-back connector, in order to perform something 
/// similar to iteration or recursion, in procedural programming.
/// A function sending a value to itself will not create any blocks, and will not be marked as blocked
/// due to the loop, and thus avoid deadlocks.
///
/// Blocks on other senders due to Constant Initializers and Loops
/// ==============================================================
/// After a function runs, its ConstantInitializers are ran, and outputs (possibly to itself) are
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
#[derive(Deserialize, Serialize, Clone)]
pub struct RunState {
    /// The vector of all functions in the flow loaded from manifest
    functions: Vec<RuntimeFunction>,
    /// blocked: HashSet<function_id> - list of functions by id that are blocked on sending
    blocked: HashSet<usize>,
    /// blocks: Vec<(blocking_id, blocking_io_number, blocked_id, blocked_flow_id)> - a list of blocks between functions
    blocks: HashSet<Block>,
    /// ready: Vec<function_id> - a list of functions by id that are ready to run
    ready: VecDeque<usize>,
    /// running: MultiMap<function_id, job_id> - a list of functions and jobs ids that are running
    running: MultiMap<usize, usize>,
    /// completed: functions that have run to completion and won't run again
    completed: HashSet<usize>,
    /// number of jobs sent for execution to date
    jobs_created: usize,
    /// limit on the number of jobs allowed to be pending to complete (i.e. running in parallel)
    max_pending_jobs: usize,
    #[cfg(feature = "debugger")]
    /// if the submission includes a request to debug the flow execution
    pub debug: bool,
    /// The timeout to be used when waiting for a job to respond
    pub job_timeout: Duration,
    /// Track which flow-function combinations are considered "busy" <flow_id, function_id>
    busy_flows: MultiMap<usize, usize>,
    /// Track which functions have finished and can be unblocked when flow goes not "busy"
    /// HashMap< <flow_id>, (function_id, vector of refilled io numbers of that function)>
    pending_unblocks: HashMap<usize, HashSet<usize>>,
}

impl RunState {
    /// Create a new `RunState` struct from the list of functions provided and the `Submission`
    /// that was sent to be executed
    pub fn new(functions: &[RuntimeFunction], submission: Submission) -> Self {
        RunState {
            functions: functions.to_vec(),
            blocked: HashSet::<usize>::new(),
            blocks: HashSet::<Block>::new(),
            ready: VecDeque::<usize>::new(),
            running: MultiMap::<usize, usize>::new(),
            completed: HashSet::<usize>::new(),
            jobs_created: 0,
            max_pending_jobs: submission.max_parallel_jobs,
            #[cfg(feature = "debugger")]
            debug: submission.debug,
            job_timeout: submission.job_timeout,
            busy_flows: MultiMap::<usize, usize>::new(),
            pending_unblocks: HashMap::<usize, HashSet<usize>>::new(),
        }
    }

    /// Get a reference to the vector of all functions
    pub fn get_functions(&self) -> &Vec<RuntimeFunction> {
        &self.functions
    }

    /// Reset all values back to initial ones to enable debugging to restart from the initial
    /// state
    #[cfg(feature = "debugger")]
    fn reset(&mut self) {
        debug!("Resetting RunState");
        for function in &mut self.functions {
            function.reset()
        }
        self.blocked.clear();
        self.blocks.clear();
        self.ready.clear();
        self.running.clear();
        self.completed.clear();
        self.jobs_created = 0;
        self.busy_flows.clear();
        self.pending_unblocks.clear();
    }

    /// The `ìnit()` function is responsible for initializing all functions, and it returns a boolean
    /// to indicate that it's inputs are fulfilled - and this information is added to the RunList
    /// to control the readiness of the Function to be executed.
    ///
    /// After init() Functions will either be:
    ///    - Ready:   an entry will be added to the `ready` list with this function's id
    ///    - Blocked: the function has all it's inputs ready and could run but a Function it sends to
    ///               has an input full already (due to being initialized during the init process)
    ///               - an entry will be added to the `blocks` list with this function's id as source_id
    ///               - an entry will be added to the `blocked` list with this function's id
    ///    - Waiting: function has at least one empty input so it cannot run. It will not added to
    ///               `ready` nor `blocked` lists, so by omission it is in the `Waiting` state.
    ///               But the `block` will be created so when later it's inputs become full the fact
    ///               it is blocked will be detected and it can move to the `blocked` state
    pub fn init(&mut self) {
        #[cfg(feature = "debugger")]
        self.reset();

        let mut inputs_ready_list = Vec::<(usize, usize)>::new();

        debug!("Initializing inputs with initializers");
        for function in &mut self.functions {
            function.init_inputs(true);
            if function.input_set_count() > 0 {
                inputs_ready_list.push((function.id(), function.get_flow_id()));
            }
        }

        // Due to initialization of some inputs other functions attempting to send to it should block
        self.create_init_blocks();

        // Put all functions that have their inputs ready and are not blocked on the `ready` list
        debug!("Readying initial functions: inputs full and not blocked on output");
        for (id, flow_id) in inputs_ready_list {
            self.new_input_set(id, flow_id, true);
        }
    }

    /// Scan through all functions and output routes for each, if the destination input is already
    /// full due to the init process, then create a block for the sender and added sender to blocked
    /// list.
    fn create_init_blocks(&mut self) {
        let mut blocks = HashSet::<Block>::new();
        let mut blocked = HashSet::<usize>::new();

        debug!("Creating any initial block entries that are needed");

        for source_function in &self.functions {
            let source_id = source_function.id();
            let source_flow_id = source_function.get_flow_id();
            let destinations = source_function.get_output_connections();
            let source_has_inputs_full = source_function.input_set_count() > 0;

            for destination in destinations {
                if destination.function_id != source_id {
                    // don't block yourself!
                    let destination_function = self.get_function(destination.function_id);
                    if destination_function.input_count(destination.io_number) > 0 {
                        trace!(
                            "\tAdded block #{} -> #{}:{}",
                            source_id,
                            destination.function_id,
                            destination.io_number
                        );
                        blocks.insert(Block::new(
                            destination.flow_id,
                            destination.function_id,
                            destination.io_number,
                            source_id,
                            source_flow_id,
                        ));
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

    /// Figure out the state of a function based on it's presence or not in the different control lists
    #[cfg(any(debug_assertions, feature = "debugger", test))]
    pub fn get_function_state(&self, function_id: usize) -> State {
        if self.ready.contains(&function_id) {
            State::Ready
        } else if self.blocked.contains(&function_id) {
            State::Blocked
        } else if self.running.contains_key(&function_id) {
            State::Running
        } else if self.completed.contains(&function_id) {
            State::Completed
        }
        else {
            State::Waiting
        }
    }

    /// Get the list of blocked function ids
    #[cfg(feature = "debugger")]
    pub fn get_blocked(&self) -> &HashSet<usize> {
        &self.blocked
    }

    /// Get a MultiMap (flow_id, function_id) of the currently running functions
    #[cfg(feature = "debugger")]
    pub fn get_running(&self) -> &MultiMap<usize, usize> {
        &self.running
    }

    /// Get the list of completed function ids
    #[cfg(feature = "debugger")]
    pub fn get_completed(&self) -> &HashSet<usize> {
        &self.completed
    }
        
    /// Get a reference to the function with `id`
    pub fn get_function(&self, id: usize) -> &RuntimeFunction {
        &self.functions[id]
    }

    /// Get a mutable reference to the function with `id`
    pub fn get_mut(&mut self, id: usize) -> &mut RuntimeFunction {
        &mut self.functions[id]
    }

    /// Get the HashSet of blocked function ids
    #[cfg(feature = "debugger")]
    pub fn get_blocks(&self) -> &HashSet<Block> {
        &self.blocks
    }

    /// Return the next job ready to be run, if there is one and there are not
    /// too many jobs already running
    pub fn next_job(&mut self) -> Option<Job> {
        if self.number_jobs_running() >= self.max_pending_jobs {
            return None;
        }

        // create a job for the function_id at the head of the ready list
        match self.ready.remove(0) {
            Some(function_id) => {
                let job = self.create_job(function_id);

                // unblock senders blocked trying to send to this function's empty inputs
                if let Some(ref j) = job {
                    self.unblock_senders(j.job_id, j.function_id, j.flow_id);
                }

                job
            }
            None => None,
        }
    }

    /// get the number of jobs created to date
    #[cfg(any(feature = "metrics", feature = "debugger"))]
    pub fn jobs_created(&self) -> usize {
        self.jobs_created
    }

    /// Given a function id, prepare a job for execution that contains the input values, the
    /// implementation and the destination functions the output should be sent to when done
    fn create_job(&mut self, function_id: usize) -> Option<Job> {
        self.jobs_created += 1;
        let job_id = self.jobs_created;

        let function = self.get_mut(function_id);

        match function.take_input_set() {
            Ok(input_set) => {
                let flow_id = function.get_flow_id();
                let implementation = function.get_implementation();
                let connections = function.get_output_connections().clone();

                Some(Job {
                    job_id,
                    function_id,
                    flow_id,
                    implementation,
                    input_set,
                    connections,
                    result: Ok((None, false)),
                })
            }
            Err(e) => {
                error!(
                    "Job #{}: Error '{}' while creating job for Function #{}",
                    job_id, e, function_id
                );
                None
            }
        }
    }

    /// Complete a Job by taking its output and updating the run-list accordingly.
    ///
    /// If other functions were blocked trying to send to this one - we can now unblock them
    /// as it has consumed it's inputs and they are free to be sent to again.
    ///
    /// Then take the output and send it to all destination IOs on different function it should be
    /// sent to, marking the source function as blocked because those others must consume the output
    /// if those other function have all their inputs, then mark them accordingly.
    pub fn complete_job(
        &mut self,
        #[cfg(feature = "metrics")] metrics: &mut Metrics,
        job: &Job,
        #[cfg(feature = "debugger")] debugger: &mut Debugger,
    ) {
        trace!(
            "Job #{}:\tCompleted by Function #{}",
            job.job_id,
            job.function_id
        );
        self.running.retain(|&_, &job_id| job_id != job.job_id);
        #[cfg(debug_assertions)]
        let job_id = job.job_id;

        if let Ok(result) = &job.result {
            let output_value = &result.0;
            let function_can_run_again = result.1;
            let mut loopback_value_sent = false;

            #[cfg(feature = "debugger")]
            let function_name = self.get_function(job.function_id).name();

            if let Some(output_v) = output_value {
            #[cfg(feature = "debugger")]
                debug!("Job #{}: Function #{} '{}' {:?} -> {:?}", job.job_id, job.function_id,
                    function_name, job.input_set,  output_v);
            #[cfg(not(feature = "debugger"))]
                debug!("Job #{}: Function #{} {:?} -> {:?}", job.job_id, job.function_id,
                    job.input_set,  output_v);

                for destination in &job.connections {
                    let value_to_send = match &destination.source {
                        Output(route) => output_v.pointer(route),
                        Input(index) => job.input_set.get(*index),
                    };

                    if let Some(value) = value_to_send {
                        if job.function_id == destination.function_id {
                            loopback_value_sent = true;
                        }
                        self.send_value(
                            job.function_id,
                            job.flow_id,
                            destination,
                            value,
                            #[cfg(feature = "metrics")]
                            metrics,
                            #[cfg(feature = "debugger")]
                            debugger,
                        );
                    } else {
                        trace!(
                            "Job #{}:\t\tNo value found at '{}'",
                            job.job_id, &destination.source
                        );
                    }
                }
            } else {
            #[cfg(feature = "debugger")]
                debug!("Job #{}: Function #{} '{}' ({:?})", job.job_id, job.function_id,
                    function_name, job.input_set);
            #[cfg(not(feature = "debugger"))]
                debug!("Job #{}: Function #{} ({:?})", job.job_id, job.function_id,
                    job.input_set);
            }

            // if the function can run again, then refill inputs from any possible initializers
            if function_can_run_again {
                self.refill_inputs(job.function_id, job.flow_id, loopback_value_sent);
            } else {
                self.mark_as_completed(job.function_id);
            }

            self.remove_from_busy(job.function_id);

            // need to do flow unblocks as that could affect other functions even if this one cannot run again
            self.unblock_flows(job.flow_id, job.job_id);
        }

        #[cfg(debug_assertions)]
        self.check_invariants(job_id);
    }

    // Take a json data value and return the array order for it
    fn array_order(value: &Value) -> i32 {
        match value {
            Value::Array(array) if !array.is_empty() => 1 + Self::array_order(&array[0]),
            Value::Array(array) if array.is_empty() => 1,
            _ => 0,
        }
    }

    // Do the necessary serialization of an array to values, or wrapping of a value into an array
    // in order to convert the value on expected by the destination, if possible, send the value
    // and return true. If the conversion cannot be done and no value is sent, return false.
    fn type_convert_and_send(
        function: &mut RuntimeFunction,
        connection: &OutputConnection,
        value: &Value,
    ) -> bool {
        if connection.is_generic() {
            function.send(connection.io_number, connection.get_priority(), value);
        } else {
            match (
                (Self::array_order(value) - connection.destination_array_order),
                value,
            ) {
                (0, _) => function.send(connection.io_number,
                                        connection.get_priority(),  value),
                (1, Value::Array(array)) => function.send_iter(connection.io_number,
                                                               connection.get_priority(), array),
                (2, Value::Array(array_2)) => {
                    for array in array_2.iter() {
                        if let Value::Array(sub_array) = array {
                            function.send_iter(connection.io_number,
                                               connection.get_priority(), sub_array)
                        }
                    }
                }
                (-1, _) => function.send(connection.io_number,
                                         connection.get_priority(), &json!([value])),
                (-2, _) => function.send(connection.io_number,
                                         connection.get_priority(), &json!([[value]])),
                _ => {
                    error!("Unable to handle difference in array order");
                    return false;
                },
            }
        }
        true // a value was sent!
    }

    /*
        Send a value produced as part of an output of running a job to a destination function on
        a specific input, update the metrics and potentially enter the debugger
    */
    fn send_value(
        &mut self,
        source_id: usize,
        source_flow_id: usize,
        connection: &OutputConnection,
        output_value: &Value,
        #[cfg(feature = "metrics")] metrics: &mut Metrics,
        #[cfg(feature = "debugger")] debugger: &mut Debugger,
    ) {
        let route_str = match &connection.source {
            Output(route) if route.is_empty() => "".into(),
            Output(route) => format!(" from output route '{}'", route),
            Input(index) => format!(" from Job value at input #{}", index),
        };

        let loopback = source_id == connection.function_id;

        if loopback {
            trace!("\t\tFunction #{source_id} loopback of '{}'{} to Self:{}",
                    output_value, route_str, connection.io_number);
        } else {
            trace!("\t\tFunction #{source_id} sending '{}'{} to Function #{}:{}",
                    output_value, route_str, connection.function_id, connection.io_number);
        };

        #[cfg(feature = "debugger")]
        if let Output(route) = &connection.source {
            debugger.check_prior_to_send(
                self,
                source_id,
                route,
                output_value,
                connection.function_id,
                connection.io_number,
            );
        }

        let function = self.get_mut(connection.function_id);
        let count_before = function.input_set_count();
        Self::type_convert_and_send(function, connection, output_value);

        #[cfg(feature = "metrics")]
        metrics.increment_outputs_sent(); // not distinguishing array serialization / wrapping etc

        let block = function.input_count(connection.io_number) > 0;
        let new_input_set_available = function.input_set_count() > count_before;

        // for the case when a function is sending to itself:
        // - avoid blocking on itself
        // - delay determining if it should be in the blocked or ready lists)
        //   until it has sent all it's other outputs as it might be blocked by another function.
        if block && !loopback {
            // TODO pass in destination and combine Block and OutputConnection?
            self.create_block(
                connection.flow_id,
                connection.function_id,
                connection.io_number,
                source_id,
                source_flow_id,
                #[cfg(feature = "debugger")]
                debugger,
            );
        }

        if new_input_set_available && !loopback {
            self.new_input_set(connection.function_id, connection.flow_id, true);
        }
    }

    // Refresh any inputs that have initializers on them, and update the state if inputs are now full
    fn refill_inputs(&mut self, function_id: usize, flow_id: usize, loopback_value_sent: bool) {
        let function = self.get_mut(function_id);

        let _count_before = function.input_set_count();

        let input_initialized = function.init_inputs(false);

        if function.input_set_count() > 0 {
            self.new_input_set(
                function_id,
                flow_id,
                input_initialized || loopback_value_sent,
            );
        }
    }

    /// Start executing `Job`
    pub fn start(&mut self, job: &Job) {
        self.running.insert(job.function_id, job.job_id);
    }

    /// Get the set of (blocking_function_id, function's IO number causing the block)
    /// of blockers for a specific function of `id`
    #[cfg(feature = "debugger")]
    pub fn get_output_blockers(&self, id: usize) -> Vec<(usize, usize)> {
        let mut blockers = vec![];

        for block in &self.blocks {
            if block.blocked_id == id {
                blockers.push((block.blocking_id, block.blocking_io_number));
            }
        }

        blockers
    }

    /// Return how many jobs are currently running
    pub fn number_jobs_running(&self) -> usize {
        let mut num_running_jobs = 0;
        for (_, vector) in self.running.iter_all() {
            num_running_jobs += vector.len()
        }
        num_running_jobs
    }

    /// Return how many jobs are ready to be run, but not running yet
    pub fn number_jobs_ready(&self) -> usize {
        self.ready.len()
    }

    /// An input blocker is another function that is the only function connected to an empty input
    /// of target function, and which is not ready to run, hence target function cannot run.
    #[cfg(feature = "debugger")]
    pub fn get_input_blockers(&self, target_id: usize) -> Vec<(usize, usize)> {
        let mut input_blockers = vec![];
        let target_function = self.get_function(target_id);

        // for each empty input of the target function
        for (target_io, input) in target_function.inputs().iter().enumerate() {
            if input.count() == 0 {
                let mut senders = Vec::<(usize, usize)>::new();

                // go through all functions to see if sends to the target function on input
                for sender_function in &self.functions {
                    // if the sender function is not ready to run
                    if !self.ready.contains(&sender_function.id()) {
                        // for each output route of sending function, see if it is sending to the target function and input
                        //(ref _output_route, destination_id, io_number, _destination_path)
                        for destination in sender_function.get_output_connections() {
                            if (destination.function_id == target_id)
                                && (destination.io_number == target_io)
                            {
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

    /// Save the fact that a new set of inputs are available for processing at the Function's inputs
    /// so it maybe ready to run (if not blocked sending on it's output)
    fn new_input_set(&mut self, id: usize, flow_id: usize, value_sent: bool) {
        // TODO I think this first part should maybe be somewhere else - a block between this function
        // and the one it wants to send to exists - but until now it did not have inputs and couldn't
        // go ready. Now it has inputs and could run, if not blocked, so it's added to the blocked list
        // TODO if we predicate this on "value_sent" also then it breaks matrix_mult sample
        if self.blocked_sending(id) {
            trace!(
                "\t\t\tFunction #{} blocked on output. Added to 'Blocked' list",
                id
            );
            // so put it on the blocked list
            self.blocked.insert(id);
        } else {
            // If a value was sent to the function (from another, from initializer or from loopback) then make ready
            // If the function has inputs backed-up and is not ready, then make ready
            if value_sent {
                trace!(
                    "\t\t\tFunction #{} not blocked on output. Added to 'Ready' list",
                    id
                );
                self.mark_ready(id, flow_id);
            }
        }
    }

    /// Mark a function "ready" to run, by adding it's id to the ready list
    fn mark_ready(&mut self, function_id: usize, flow_id: usize) {
        self.ready.push_back(function_id);
        self.busy_flows.insert(flow_id, function_id);
    }

    /// See if there is any block where the blocked function is the one we're looking for
    fn blocked_sending(&self, id: usize) -> bool {
        for block in &self.blocks {
            if block.blocked_id == id {
                return true;
            }
        }
        false
    }

    /// Return how many functions exist in this flow being executed
    #[cfg(any(feature = "debugger", feature = "metrics"))]
    pub fn num_functions(&self) -> usize {
        self.functions.len()
    }

    /// The function blocker_function_id in flow blocked_flow_id has completed execution and so
    /// is a candidate to send to from other functions that were blocked sending to it previously.
    ///
    /// But we don't want to unblock them to send to it, until all other functions inside this flow
    /// are idle, and hence the flow becomes idle.
    pub fn unblock_senders(
        &mut self,
        job_id: usize,
        blocker_function_id: usize,
        blocker_flow_id: usize,
    ) {
        // delete blocks to this function from within the same flow
        let flow_internal_blocks = |block: &Block| block.blocking_flow_id == block.blocked_flow_id;

        self.unblock_senders_to_function(blocker_function_id, flow_internal_blocks);

        // Add this function to the pending unblock list for later when flow goes idle - ensure entry is unique
        let mut new_set = HashSet::new();
        new_set.insert(blocker_function_id);
        let set = self
            .pending_unblocks
            .entry(blocker_flow_id)
            .or_insert(new_set);
        set.insert(blocker_function_id);
        trace!(
            "Job #{}:\t\tAdded a pending_unblock -> #{}({})",
            job_id,
            blocker_function_id,
            blocker_flow_id
        );
    }

    /// Detect which flows have gone inactive and remove pending unblocks for functions in it
    fn unblock_flows(&mut self, blocker_flow_id: usize, job_id: usize) {
        let flow_external_blocks = |block: &Block| block.blocking_flow_id != block.blocked_flow_id;

        // if flow is now idle, remove any blocks on sending to functions in the flow
        if self.busy_flows.get(&blocker_flow_id).is_none() {
            trace!(
                "Job #{}:\tFlow #{} is now idle, so removing pending_unblocks for flow #{}",
                job_id,
                blocker_flow_id,
                blocker_flow_id
            );

            if let Some(pending_unblocks) = self.pending_unblocks.remove(&blocker_flow_id) {
                trace!(
                    "Job #{}:\tRemoving pending unblocks to functions in Flow #{} from other flows",
                    job_id,
                    blocker_flow_id
                );
                for unblock_function_id in pending_unblocks {
                    self.unblock_senders_to_function(unblock_function_id, flow_external_blocks);
                }
            }
        }
    }

    /// Mark a function (via its ID) as having run to completion 
    fn mark_as_completed(&mut self, function_id: usize) {
        self.completed.insert(function_id);
    }

    /// Remove ONE entry of <flow_id, function_id> from the busy_flows multi-map
    fn remove_from_busy(&mut self, blocker_function_id: usize) {
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

    /// unblock all functions that were blocked trying to send to blocker_function_id by removing all entries
    /// in the `blocks` list where the first value (blocking_id) matches blocker_function_id.
    ///
    /// Once each is unblocked, if it's inputs are full, then it is ready to be run again,
    /// so mark as ready
    fn unblock_senders_to_function<F>(&mut self, blocker_function_id: usize, f: F)
    where
        F: Fn(&Block) -> bool,
    {
        let mut unblock_list = vec![];

        self.blocks.retain(|block| {
            if (block.blocking_id == blocker_function_id) && f(block) {
                unblock_list.push((block.blocked_id, block.blocked_flow_id));
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
                trace!(
                    "\t\t\t\tFunction #{} \
                removed from 'blocked' list",
                    unblocked_id
                );
                self.blocked.remove(&unblocked_id);

                if self.get_function(unblocked_id).input_set_count() > 0 {
                    trace!(
                        "\t\t\t\tFunction #{} has inputs ready, so added to 'ready' list",
                        unblocked_id
                    );
                    self.mark_ready(unblocked_id, unblocked_flow_id);
                }
            }
        }
    }

    /// Create a 'block" indicating that function 'blocked_id' cannot run as it has an output
    /// destination to an input on function 'blocking_id' that is already full.
    fn create_block(
        &mut self,
        blocking_flow_id: usize,
        blocking_id: usize,
        blocking_io_number: usize,
        blocked_id: usize,
        blocked_flow_id: usize,
        #[cfg(feature = "debugger")] debugger: &mut Debugger,
    ) {
        let block = Block::new(
            blocking_flow_id,
            blocking_id,
            blocking_io_number,
            blocked_id,
            blocked_flow_id,
        );
        trace!("\t\t\t\t\tCreating Block {:?}", block);

        if !self.blocks.contains(&block) {
            #[cfg(not(feature = "debugger"))]
            self.blocks.insert(block);
            #[cfg(feature = "debugger")]
            self.blocks.insert(block.clone());
            #[cfg(feature = "debugger")]
            debugger.check_on_block_creation(self, &block);
        }
    }

    #[cfg(debug_assertions)]
    fn runtime_error(&self, job_id: usize, message: &str, file: &str, line: u32) {
        error!(
            "Job #{}: Runtime error: at file: {}, line: {}\n\t\t{}",
            job_id, file, line, message
        );
        error!("Job #{}: Error State - {}", job_id, self);
    }

    /// Check a number of "invariants" i.e. unbreakable rules about the state.
    /// If one is found to be broken, report a runtime error explaining it, which may
    /// trigger entry into the debugger.
    #[cfg(debug_assertions)]
    fn check_invariants(&mut self, job_id: usize) {
        for function in &self.functions {
            match self.get_function_state(function.id()) {
                State::Ready => {
                    if !self.busy_flows.contains_key(&function.get_flow_id()) {
                        return self.runtime_error(
                            job_id,
                            &format!(
                                "Function #{} is Ready, but Flow #{} is not busy",
                                function.id(),
                                function.get_flow_id()
                            ),
                            file!(),
                            line!(),
                        );
                    }
                }
                State::Running => {
                    if !self.busy_flows.contains_key(&function.get_flow_id()) {
                        return self.runtime_error(
                            job_id,
                            &format!(
                                "Function #{} is Running, but Flow #{} is not busy",
                                function.id(),
                                function.get_flow_id()
                            ),
                            file!(),
                            line!(),
                        );
                    }
                }
                State::Blocked => {
                    if !self.blocked_sending(function.id()) {
                        return self.runtime_error(
                            job_id,
                            &format!(
                                "Function #{} is in Blocked state, but no block exists",
                                function.id()
                            ),
                            file!(),
                            line!(),
                        );
                    }
                }
                State::Waiting => {},
                State::Completed => {
                    // If completed, should not be in any of the other states
                    if self.ready.contains(&function.id()) ||
                    self.blocked.contains(&function.id()) ||
                    self.running.contains_key(&function.id())
                    {
                        return self.runtime_error(
                            job_id,
                            &format!(
                                "Function #{} has Completed, but also appears as Ready or Blocked or Running",
                                function.id(),
                            ),
                            file!(),
                            line!(),
                        );
                    }
                },
            }

            // State::Running is because functions with initializers auto-refill
            // So they will show as inputs full, but not Ready or Blocked
            let state = self.get_function_state(function.id());
            if (!function.inputs().is_empty())
                && (function.input_set_count() > 0)
                && !(state == State::Ready || state == State::Blocked || state == State::Running)
            {
                #[cfg(feature = "debugger")]
                error!("{}", function);
                return self.runtime_error(
                    job_id,
                    &format!(
                        "Function #{} inputs have data, but it is not Ready or Blocked or Running",
                        function.id()
                    ),
                    file!(),
                    line!(),
                );
            }
        }

        // Check block invariants
        for block in &self.blocks {
            // function should not be blocked on itself
            if block.blocked_id == block.blocking_id {
                return self.runtime_error(
                    job_id,
                    &format!(
                        "Block {} has same Function id as blocked and blocking",
                        block
                    ),
                    file!(),
                    line!(),
                );
            }

            // For each block on a destination function, then either that input should be full or
            // the function should be running in parallel with the one that just completed
            // or it's flow should be busy and there should be a pending unblock on it
            if let Some(function) = self.functions.get(block.blocking_id) {
                if !(function.input_count(block.blocking_io_number) > 0
                    || (self.busy_flows.contains_key(&block.blocking_flow_id)
                        && self.pending_unblocks.contains_key(&block.blocking_flow_id)))
                {
                    return self.runtime_error(job_id,
                                              &format!("Block {} exists for function #{}, but Function #{}:{} input is not full",
                                                       block, block.blocking_id, block.blocking_id, block.blocking_io_number),
                                              file!(), line!());
                }
            }
        }

        // Check pending unblock invariants
        for pending_unblock_flow_id in self.pending_unblocks.keys() {
            // flow it's in must be busy
            if !self.busy_flows.contains_key(pending_unblock_flow_id) {
                return self.runtime_error(
                    job_id,
                    &format!(
                        "Pending Unblock exists for Flow #{}, but it is not busy",
                        pending_unblock_flow_id
                    ),
                    file!(),
                    line!(),
                );
            }
        }

        // Check busy flow invariants
        for (flow_id, function_id) in self.busy_flows.iter() {
            let state = self.get_function_state(*function_id);
            if !(state == State::Ready || state == State::Running) {
                return self.runtime_error(job_id, &format!("Busy flow entry exists for Function #{} in Flow #{} but it's state is {:?}",
                                                           function_id, flow_id, state),
                                          file!(), line!());
            }
        }
    }
}

impl fmt::Display for RunState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "RunState:")?;
        writeln!(f, "       Jobs Created: {}", self.jobs_created)?;
        writeln!(f, ". Functions Blocked: {:?}", self.blocked)?;
        writeln!(f, "             Blocks: {:?}", self.blocks)?;
        writeln!(f, "    Functions Ready: {:?}", self.ready)?;
        writeln!(f, "  Functions Running: {:?}", self.running)?;
        writeln!(f, "Functions Completed: {:?}", self.completed)?;
        writeln!(f, "         Flows Busy: {:?}", self.busy_flows)?;
        write!(f, "     Pending Unblocks: {:?}", self.pending_unblocks)
    }
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use serde_json::json;
    use serde_json::Value;

    use flowcore::{Implementation, RunAgain};
    use flowcore::errors::Result;
    use flowcore::model::input::Input;
    use flowcore::model::input::InputInitializer::Once;
    use flowcore::model::output_connection::{OutputConnection, Source};
    use flowcore::model::runtime_function::RuntimeFunction;

    #[cfg(feature = "debugger")]
    use crate::block::Block;
    #[cfg(feature = "debugger")]
    use crate::debug_command::DebugCommand;
    #[cfg(feature = "debugger")]
    use crate::debugger::Debugger;
    #[cfg(feature = "debugger")]
    use crate::run_state::{RunState, State};
    #[cfg(feature = "debugger")]
    use crate::server::DebugServer;

    use super::Job;

    #[derive(Debug)]
    struct TestImpl {}

    impl Implementation for TestImpl {
        fn run(&self, _inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
            unimplemented!()
        }
    }

    fn test_impl() -> Arc<dyn Implementation> {
        Arc::new(TestImpl {})
    }

    fn test_function_a_to_b_not_init() -> RuntimeFunction {
        let connection_to_f1 = OutputConnection::new(
            Source::default(),
            1,
            0,
            0,
            0,
            false,
            "/fB".to_string(),
            #[cfg(feature = "debugger")]
            String::default(),
            0,
        );

        RuntimeFunction::new(
            #[cfg(feature = "debugger")]
            "fA",
            #[cfg(feature = "debugger")]
            "/fA",
            "file://fake/test",
            vec![Input::new(
                        #[cfg(feature = "debugger")] "",
                            &None)],
            0,
            0,
            &[connection_to_f1],
            false,
        ) // outputs to fB:0
    }

    fn test_function_a_to_b() -> RuntimeFunction {
        let connection_to_f1 = OutputConnection::new(
            Source::default(),
            1,
            0,
            0,
            0,
            false,
            "/fB".to_string(),
            #[cfg(feature = "debugger")]
            String::default(),
            0,
        );
        RuntimeFunction::new(
            #[cfg(feature = "debugger")]
            "fA",
            #[cfg(feature = "debugger")]
            "/fA",
            "file://fake/test",
            vec![Input::new(
                            #[cfg(feature = "debugger")] "",
                            &Some(Once(json!(1))))],
            0,
            0,
            &[connection_to_f1],
            false,
        ) // outputs to fB:0
    }

    fn test_function_a_init() -> RuntimeFunction {
        RuntimeFunction::new(
            #[cfg(feature = "debugger")]
            "fA",
            #[cfg(feature = "debugger")]
            "/fA",
            "file://fake/test",
            vec![Input::new(
                #[cfg(feature = "debugger")] "",
                &Some(Once(json!(1))))],
            0,
            0,
            &[],
            false,
        )
    }

    fn test_function_b_not_init() -> RuntimeFunction {
        RuntimeFunction::new(
            #[cfg(feature = "debugger")]
            "fB",
            #[cfg(feature = "debugger")]
            "/fB",
            "file://fake/test",
            vec![Input::new(
                #[cfg(feature = "debugger")] "",
                &None)],
            1,
            0,
            &[],
            false,
        )
    }

    fn test_function_b_init() -> RuntimeFunction {
        RuntimeFunction::new(
            #[cfg(feature = "debugger")]
            "fB",
            #[cfg(feature = "debugger")]
            "/fB",
            "file://fake/test",
            vec![Input::new(
                #[cfg(feature = "debugger")] "",
                &Some(Once(json!(1))))],
            1,
            0,
            &[],
            false,
        )
    }

    fn test_output(source_function_id: usize, destination_function_id: usize) -> Job {
        let out_conn = OutputConnection::new(
            Source::default(),
            destination_function_id,
            0,
            0,
            0,
            false,
            String::default(),
            #[cfg(feature = "debugger")]
            String::default(),
            0,
        );
        Job {
            job_id: 1,
            function_id: source_function_id,
            flow_id: 0,
            implementation: test_impl(),
            input_set: vec![json!(1)],
            result: Ok((Some(json!(1)), true)),
            connections: vec![out_conn],
        }
    }

    #[cfg(feature = "debugger")]
    struct DummyServer;
    #[cfg(feature = "debugger")]
    impl DebugServer for DummyServer {
        fn start(&mut self) {}
        fn job_breakpoint(&mut self, _job: &Job, _function: &RuntimeFunction, _state: State) {}
        fn block_breakpoint(&mut self, _block: &Block) {}
        fn send_breakpoint(&mut self, _: &str, _source_process_id: usize, _output_route: &str, _value: &Value,
                           _destination_id: usize, _destination_name:&str, _input_name: &str, _input_number: usize) {}
        fn job_error(&mut self, _job: &Job) {}
        fn job_completed(&mut self, _job: &Job) {}
        fn blocks(&mut self, _blocks: Vec<Block>) {}
        fn outputs(&mut self, _output: Vec<OutputConnection>) {}
        fn input(&mut self, _input: Input) {}
        fn function_list(&mut self, _functions: &[RuntimeFunction]) {}
        fn function_state(&mut self, _function: RuntimeFunction, _function_state: State) {}
        fn run_state(&mut self, _run_state: &RunState) {}
        fn message(&mut self, _message: String) {}
        fn panic(&mut self, _state: &RunState, _error_message: String) {}
        fn debugger_exiting(&mut self) {}
        fn debugger_resetting(&mut self) {}
        fn debugger_error(&mut self, _error: String) {}
        fn execution_starting(&mut self) {}
        fn execution_ended(&mut self) {}
        fn get_command(&mut self, _state: &RunState) -> flowcore::errors::Result<DebugCommand> {
            todo!()
        }
    }

    #[cfg(feature = "debugger")]
    fn dummy_debugger(server: &mut dyn DebugServer) -> Debugger {
        Debugger::new(server)
    }

    mod general_run_state_tests {
        #[cfg(feature = "debugger")]
                use std::collections::HashSet;

        #[cfg(feature = "debugger")]
                use multimap::MultiMap;
        use url::Url;

        use flowcore::model::submission::Submission;

        use super::super::RunState;

        #[test]
        fn display_run_state_test() {
            let f_a = super::test_function_a_to_b();
            let f_b = super::test_function_b_not_init();
            let functions = vec![f_a, f_b];
            let submission = Submission::new(
                &Url::parse("file:///temp/fake.toml").expect("Could not create Url"),
                1,
                #[cfg(feature = "debugger")]
                true,
            );
            let mut state = RunState::new(&functions, submission);
            state.init();

            #[cfg(any(feature = "debugger", feature = "metrics"))]
            assert_eq!(state.num_functions(), 2);

            println!("Run state: {}", state);
        }

        #[cfg(feature = "metrics")]
        #[test]
        fn jobs_created_zero_at_init() {
            let submission = Submission::new(
                &Url::parse("file:///temp/fake.toml").expect("Could not create Url"),
                1,
                #[cfg(feature = "debugger")]
                true,
            );
            let mut state = RunState::new(&[], submission);
            state.init();
            assert_eq!(0, state.jobs_created(), "At init jobs() should be 0");
            assert_eq!(0, state.number_jobs_ready());
        }

        #[cfg(feature = "debugger")]
        #[test]
        fn zero_blocks_at_init() {
            let submission = Submission::new(
                &Url::parse("file:///temp/fake.toml").expect("Could not create Url"),
                1,
                #[cfg(feature = "debugger")]
                true,
            );
            let mut state = RunState::new(&[], submission);
            state.init();
            assert_eq!(
                &HashSet::new(),
                state.get_blocks(),
                "At init get_blocks() should be empty"
            );
        }

        #[cfg(feature = "debugger")]
        #[test]
        fn zero_running_at_init() {
            let submission = Submission::new(
                &Url::parse("file:///temp/fake.toml").expect("Could not create Url"),
                1,
                #[cfg(feature = "debugger")]
                true,
            );
            let mut state = RunState::new(&[], submission);
            state.init();
            assert_eq!(
                &MultiMap::new(),
                state.get_running(),
                "At init get_running() should be empty"
            );
        }

        #[cfg(feature = "debugger")]
        #[test]
        fn zero_blocked_at_init() {
            let submission = Submission::new(
                &Url::parse("file:///temp/fake.toml").expect("Could not create Url"),
                1,
                #[cfg(feature = "debugger")]
                true,
            );
            let mut state = RunState::new(&[], submission);
            state.init();
            assert_eq!(
                &HashSet::new(),
                state.get_blocked(),
                "At init get_blocked() should be empty"
            );
        }
    }

    /********************************* State Transition Tests *********************************/
    mod state_transitions {
        use serde_json::json;
        use serial_test::serial;
        use url::Url;

        use flowcore::model::input::Input;
        use flowcore::model::input::InputInitializer::{Always, Once};
        #[cfg(feature = "metrics")]
        use flowcore::model::metrics::Metrics;
        use flowcore::model::output_connection::{OutputConnection, Source};
        use flowcore::model::output_connection::Source::Output;
        use flowcore::model::runtime_function::RuntimeFunction;
        use flowcore::model::submission::Submission;

        use crate::run_state::test::test_function_b_not_init;

        use super::super::Job;
        use super::super::RunState;
        use super::super::State;

        #[test]
        fn to_ready_1_on_init() {
            let f_a = super::test_function_a_to_b();
            let f_b = super::test_function_b_not_init();
            let functions = vec![f_a, f_b];
            let submission = Submission::new(
                &Url::parse("file:///temp/fake.toml").expect("Could not create Url"),
                1,
                #[cfg(feature = "debugger")]
                true,
            );
            let mut state = RunState::new(&functions, submission);

            // Event
            state.init();

            // Test
            assert_eq!(State::Ready, state.get_function_state(0), "f_a should be Ready");
            assert_eq!(1, state.number_jobs_ready());
            assert_eq!(
                State::Waiting,
                state.get_function_state(1),
                "f_b should be waiting for input"
            );
        }

        #[test]
        fn input_blocker() {
            let f_a = super::test_function_a_to_b_not_init();
            let f_b = super::test_function_b_not_init();
            let functions = vec![f_a, f_b];
            let submission = Submission::new(
                &Url::parse("file:///temp/fake.toml").expect("Could not create Url"),
                1,
                #[cfg(feature = "debugger")]
                true,
            );
            let mut state = RunState::new(&functions, submission);

            // Event
            state.init();

            // Test
            assert_eq!(
                State::Waiting,
                state.get_function_state(0),
                "f_a should be waiting for input"
            );
            assert_eq!(
                State::Waiting,
                state.get_function_state(1),
                "f_b should be waiting for input"
            );
            #[cfg(feature = "debugger")]
            assert!(
                state.get_input_blockers(1).contains(&(0, 0)),
                "f_b should be waiting for input from f_a"
            )
        }

        #[test]
        fn to_ready_2_on_init() {
            let f_a = super::test_function_a_to_b();
            let f_b = super::test_function_b_not_init();
            let functions = vec![f_a, f_b];
            let submission = Submission::new(
                &Url::parse("file:///temp/fake.toml").expect("Could not create Url"),
                1,
                #[cfg(feature = "debugger")]
                true,
            );
            let mut state = RunState::new(&functions, submission);

            // Event
            state.init();

            // Test
            assert_eq!(State::Ready, state.get_function_state(0), "f_a should be Ready");
        }

        #[test]
        fn to_ready_3_on_init() {
            let f_a = super::test_function_a_init();
            let functions = vec![f_a];
            let submission = Submission::new(
                &Url::parse("file:///temp/fake.toml").expect("Could not create Url"),
                1,
                #[cfg(feature = "debugger")]
                true,
            );
            let mut state = RunState::new(&functions, submission);

            // Event
            state.init();

            // Test
            assert_eq!(State::Ready, state.get_function_state(0), "f_a should be Ready");
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
            let functions = vec![f_b, f_a];
            let submission = Submission::new(
                &Url::parse("file:///temp/fake.toml").expect("Could not create Url"),
                1,
                #[cfg(feature = "debugger")]
                true,
            );
            let mut state = RunState::new(&functions, submission);

            // Event
            state.init();

            // Test
            assert_eq!(State::Ready, state.get_function_state(1), "f_b should be Ready");
            assert_eq!(
                State::Blocked,
                state.get_function_state(0),
                "f_a should be in Blocked state"
            );
            #[cfg(feature = "debugger")]
            assert!(
                state.get_output_blockers(0).contains(&(1, 0)),
                "f_a should be blocked by f_b, input 0"
            );
        }

        fn test_function_a_not_init() -> RuntimeFunction {
            RuntimeFunction::new(
                #[cfg(feature = "debugger")]
                "fA",
                #[cfg(feature = "debugger")]
                "/fA",
                "file://fake/test",
                vec![Input::new(
                    #[cfg(feature = "debugger")] "",
                    &None)],
                0,
                0,
                &[],
                false,
            )
        }

        #[test]
        fn to_waiting_on_init() {
            let f_a = test_function_a_not_init();
            let functions = vec![f_a];
            let submission = Submission::new(
                &Url::parse("file:///temp/fake.toml").expect("Could not create Url"),
                1,
                #[cfg(feature = "debugger")]
                true,
            );
            let mut state = RunState::new(&functions, submission);

            // Event
            state.init();

            // Test
            assert_eq!(State::Waiting, state.get_function_state(0), "f_a should be Waiting");
        }

        #[test]
        fn ready_to_running_on_next() {
            let f_a = super::test_function_a_init();
            let functions = vec![f_a];
            let submission = Submission::new(
                &Url::parse("file:///temp/fake.toml").expect("Could not create Url"),
                1,
                #[cfg(feature = "debugger")]
                true,
            );
            let mut state = RunState::new(&functions, submission);
            state.init();
            assert_eq!(State::Ready, state.get_function_state(0), "f_a should be Ready");

            // Event
            let job = state.next_job().expect("Couldn't get next job");
            assert_eq!(
                0, job.function_id,
                "next_job() should return function_id = 0"
            );
            state.start(&job);

            // Test
            assert_eq!(State::Running, state.get_function_state(0), "f_a should be Running");
        }

        #[test]
        fn unready_not_to_running_on_next() {
            let f_a = test_function_a_not_init();
            let functions = vec![f_a];
            let submission = Submission::new(
                &Url::parse("file:///temp/fake.toml").expect("Could not create Url"),
                1,
                #[cfg(feature = "debugger")]
                true,
            );
            let mut state = RunState::new(&functions, submission);
            state.init();
            assert_eq!(State::Waiting, state.get_function_state(0), "f_a should be Waiting");

            // Event
            assert!(state.next_job().is_none(), "next_job() should return None");

            // Test
            assert_eq!(State::Waiting, state.get_function_state(0), "f_a should be Waiting");
        }

        #[serial]
        #[test]
        fn blocked_to_ready_on_done() {
            let f_a = super::test_function_a_to_b();
            let f_b = super::test_function_b_init();
            let functions = vec![f_a, f_b];
            let submission = Submission::new(
                &Url::parse("file:///temp/fake.toml").expect("Could not create Url"),
                1,
                #[cfg(feature = "debugger")]
                true,
            );
            let mut state = RunState::new(&functions, submission);
            #[cfg(feature = "metrics")]
            let mut metrics = Metrics::new(2);
            #[cfg(feature = "debugger")]
            let mut server = super::DummyServer{};
            #[cfg(feature = "debugger")]
            let mut debugger = super::dummy_debugger(&mut server);

            // Initial state
            state.init();
            assert_eq!(State::Ready, state.get_function_state(1), "f_b should be Ready");
            assert_eq!(
                State::Blocked,
                state.get_function_state(0),
                "f_a should be in Blocked state, by f_b"
            );

            // First job
            let job = state.next_job().expect("Couldn't get next job");
            assert_eq!(
                1, job.function_id,
                "next() should return function_id=1 (f_b) for running"
            );
            state.start(&job);
            assert_eq!(State::Running, state.get_function_state(1), "f_b should be Running");

            // Event
            let output = super::test_output(1, 0);
            state.complete_job(
                #[cfg(feature = "metrics")]
                &mut metrics,
                &output,
                #[cfg(feature = "debugger")]
                &mut debugger,
            );

            // Test
            assert_eq!(State::Ready, state.get_function_state(0), "f_a should be Ready");
        }

        #[test]
        #[serial]
        fn output_not_found() {
            let f_a = super::test_function_a_to_b();
            let f_b = super::test_function_b_init();
            let functions = vec![f_a, f_b];
            let submission = Submission::new(
                &Url::parse("file:///temp/fake.toml").expect("Could not create Url"),
                1,
                #[cfg(feature = "debugger")]
                true,
            );
            let mut state = RunState::new(&functions, submission);
            #[cfg(feature = "metrics")]
            let mut metrics = Metrics::new(2);
            #[cfg(feature = "debugger")]
                let mut server = super::DummyServer{};
            #[cfg(feature = "debugger")]
                let mut debugger = super::dummy_debugger(&mut server);

            // Initial state
            state.init();
            assert_eq!(State::Ready, state.get_function_state(1), "f_b should be Ready");
            assert_eq!(
                State::Blocked,
                state.get_function_state(0),
                "f_a should be in Blocked state, by f_b"
            );

            let job = state.next_job().expect("Couldn't get next job");
            assert_eq!(
                1, job.function_id,
                "next() should return function_id=1 (f_b) for running"
            );
            state.start(&job);
            assert_eq!(State::Running, state.get_function_state(1), "f_b should be Running");

            // Event
            let mut output = super::test_output(1, 0);

            // Modify test output to use a route that doesn't exist
            let no_such_out_conn = OutputConnection::new(
                Output("/fake".into()),
                0,
                0,
                0,
                0,
                false,
                String::default(),
                #[cfg(feature = "debugger")]
                String::default(),
                0,
            );
            output.connections = vec![no_such_out_conn];

            state.complete_job(
                #[cfg(feature = "metrics")]
                &mut metrics,
                &output,
                #[cfg(feature = "debugger")]
                &mut debugger,
            );

            // Test
            assert_eq!(State::Ready, state.get_function_state(0), "f_a should be Ready");
        }

        fn test_job() -> Job {
            Job {
                job_id: 1,
                function_id: 0,
                flow_id: 0,
                implementation: super::test_impl(),
                input_set: vec![json!(1)],
                result: Ok((None, true)),
                connections: vec![],
            }
        }

        #[test]
        #[serial]
        fn running_to_ready_on_done() {
            let f_a = RuntimeFunction::new(
                #[cfg(feature = "debugger")]
                "fA",
                #[cfg(feature = "debugger")]
                "/fA",
                "file://fake/test",
                vec![Input::new(
                    #[cfg(feature = "debugger")] "",
                    &Some(Always(json!(1))))],
                0,
                0,
                &[],
                false,
            );
            let functions = vec![f_a];
            let submission = Submission::new(
                &Url::parse("file:///temp/fake.toml").expect("Could not create Url"),
                1,
                #[cfg(feature = "debugger")]
                true,
            );
            let mut state = RunState::new(&functions, submission);
            #[cfg(feature = "metrics")]
            let mut metrics = Metrics::new(1);
            #[cfg(feature = "debugger")]
                let mut server = super::DummyServer{};
            #[cfg(feature = "debugger")]
                let mut debugger = super::dummy_debugger(&mut server);

            state.init();
            assert_eq!(State::Ready, state.get_function_state(0), "f_a should be Ready");
            let job = state.next_job().expect("Couldn't get next job");
            assert_eq!(0, job.function_id, "next() should return function_id = 0");
            state.start(&job);

            assert_eq!(State::Running, state.get_function_state(0), "f_a should be Running");

            // Event
            let job = test_job();
            state.complete_job(
                #[cfg(feature = "metrics")]
                &mut metrics,
                &job,
                #[cfg(feature = "debugger")]
                &mut debugger,
            );

            // Test
            assert_eq!(
                State::Ready,
                state.get_function_state(0),
                "f_a should be Ready again"
            );
        }

        // Done: it has one input or more empty, to it can't run
        #[test]
        #[serial]
        fn running_to_waiting_on_done() {
            let f_a = super::test_function_a_init();
            let functions = vec![f_a];
            let submission = Submission::new(
                &Url::parse("file:///temp/fake.toml").expect("Could not create Url"),
                1,
                #[cfg(feature = "debugger")]
                true,
            );
            let mut state = RunState::new(&functions, submission);
            #[cfg(feature = "metrics")]
            let mut metrics = Metrics::new(1);
            #[cfg(feature = "debugger")]
                let mut server = super::DummyServer{};
            #[cfg(feature = "debugger")]
                let mut debugger = super::dummy_debugger(&mut server);

            state.init();
            assert_eq!(State::Ready, state.get_function_state(0), "f_a should be Ready");
            let job = state.next_job().expect("Couldn't get next job");
            assert_eq!(0, job.function_id, "next() should return function_id = 0");
            state.start(&job);

            assert_eq!(State::Running, state.get_function_state(0), "f_a should be Running");

            // Event
            let job = test_job();
            state.complete_job(
                #[cfg(feature = "metrics")]
                &mut metrics,
                &job,
                #[cfg(feature = "debugger")]
                &mut debugger,
            );

            // Test
            assert_eq!(
                State::Waiting,
                state.get_function_state(0),
                "f_a should be Waiting again"
            );
        }

        // Done: at least one destination input is full, so can't run  running_to_blocked_on_done
        #[test]
        #[serial]
        fn running_to_blocked_on_done() {
            let out_conn = OutputConnection::new(
                Source::default(),
                1,
                0,
                0,
                0,
                false,
                String::default(),
                #[cfg(feature = "debugger")]
                String::default(),
                0,
            );
            let f_a = RuntimeFunction::new(
                #[cfg(feature = "debugger")]
                "fA",
                #[cfg(feature = "debugger")]
                "/fA",
                "file://fake/test",
                vec![Input::new(
                    #[cfg(feature = "debugger")] "",
                    &Some(Always(json!(1))))],
                0,
                0,
                &[out_conn],
                false,
            ); // outputs to fB:0
            let f_b = test_function_b_not_init();
            let functions = vec![f_a, f_b];
            let submission = Submission::new(
                &Url::parse("file:///temp/fake.toml").expect("Could not create Url"),
                1,
                #[cfg(feature = "debugger")]
                true,
            );
            let mut state = RunState::new(&functions, submission);
            #[cfg(feature = "metrics")]
            let mut metrics = Metrics::new(1);
            #[cfg(feature = "debugger")]
                let mut server = super::DummyServer{};
            #[cfg(feature = "debugger")]
                let mut debugger = super::dummy_debugger(&mut server);

            state.init();

            assert_eq!(State::Ready, state.get_function_state(0), "f_a should be Ready");

            let job = state.next_job().expect("Couldn't get next job");
            assert_eq!(
                0, job.function_id,
                "next() should return function_id=0 (f_a) for running"
            );
            state.start(&job);

            assert_eq!(State::Running, state.get_function_state(0), "f_a should be Running");

            // Event
            let output = super::test_output(0, 1);
            state.complete_job(
                #[cfg(feature = "metrics")]
                &mut metrics,
                &output,
                #[cfg(feature = "debugger")]
                &mut debugger,
            );

            // Test f_a should transition to Blocked on f_b
            assert_eq!(State::Blocked, state.get_function_state(0), "f_a should be Blocked");
        }

        #[test]
        #[serial]
        fn waiting_to_ready_on_input() {
            let f_a = test_function_a_not_init();
            let out_conn = OutputConnection::new(
                Source::default(),
                0,
                0,
                0,
                0,
                false,
                String::default(),
                #[cfg(feature = "debugger")]
                String::default(),
                0,
            );
            let f_b = RuntimeFunction::new(
                #[cfg(feature = "debugger")]
                "fB",
                #[cfg(feature = "debugger")]
                "/fB",
                "file://fake/test",
                vec![Input::new(
                    #[cfg(feature = "debugger")] "",
                    &None)],
                1,
                0,
                &[out_conn],
                false,
            );
            let functions = vec![f_a, f_b];
            let submission = Submission::new(
                &Url::parse("file:///temp/fake.toml").expect("Could not create Url"),
                1,
                #[cfg(feature = "debugger")]
                true,
            );
            let mut state = RunState::new(&functions, submission);
            #[cfg(feature = "metrics")]
            let mut metrics = Metrics::new(1);
            #[cfg(feature = "debugger")]
                let mut server = super::DummyServer{};
            #[cfg(feature = "debugger")]
                let mut debugger = super::dummy_debugger(&mut server);

            state.init();
            assert_eq!(State::Waiting, state.get_function_state(0), "f_a should be Waiting");

            // Event run f_b which will send to f_a
            let output = super::test_output(1, 0);
            state.complete_job(
                #[cfg(feature = "metrics")]
                &mut metrics,
                &output,
                #[cfg(feature = "debugger")]
                &mut debugger,
            );

            // Test
            assert_eq!(State::Ready, state.get_function_state(0), "f_a should be Ready");
        }

        /*
            fA (#0) has an input but not initialized, outputs to #1 (fB)
            fB (#1) has an input with a ConstantInitializer, outputs back to #0 (fA)
        */
        #[test]
        #[serial]
        fn waiting_to_blocked_on_input() {
            let f_a = super::test_function_a_to_b_not_init();
            let connection_to_f0 = OutputConnection::new(
                Source::default(),
                0,
                0,
                0,
                0,
                false,
                String::default(),
                #[cfg(feature = "debugger")]
                String::default(),
                0,
            );
            let f_b = RuntimeFunction::new(
                #[cfg(feature = "debugger")]
                "fB",
                #[cfg(feature = "debugger")]
                "/fB",
                "file://fake/test",
                vec![Input::new(
                    #[cfg(feature = "debugger")] "",
                    &Some(Always(json!(1))))],
                1,
                0,
                &[connection_to_f0],
                false,
            );
            let functions = vec![f_a, f_b];
            let submission = Submission::new(
                &Url::parse("file:///temp/fake.toml").expect("Could not create Url"),
                1,
                #[cfg(feature = "debugger")]
                true,
            );
            let mut state = RunState::new(&functions, submission);
            #[cfg(feature = "metrics")]
            let mut metrics = Metrics::new(1);
            #[cfg(feature = "debugger")]
                let mut server = super::DummyServer{};
            #[cfg(feature = "debugger")]
                let mut debugger = super::dummy_debugger(&mut server);

            state.init();

            assert_eq!(state.get_function_state(1), State::Ready, "f_b should be Ready");
            assert_eq!(
                state.get_function_state(0),
                State::Waiting,
                "f_a should be in Waiting"
            );

            assert_eq!(
                state.next_job().expect("Couldn't get next job").function_id,
                1,
                "next() should return function_id=1 (f_b) for running"
            );

            // create output from f_b as if it had run - will send to f_a
            let output = super::test_output(1, 0);
            state.complete_job(
                #[cfg(feature = "metrics")]
                &mut metrics,
                &output,
                #[cfg(feature = "debugger")]
                &mut debugger,
            );

            // Test
            assert_eq!(state.get_function_state(0), State::Ready, "f_a should be Ready");
        }

        /*
            This tests that if a function that has a loop back sending to itself, runs the first time
            due to a OnceInitializer, that after running it sends output back to itself and is ready
            (not waiting for an input from elsewhere and no deadlock due to blocking itself occurs
        */
        #[test]
        #[serial]
        fn not_block_on_self() {
            let connection_to_0 = OutputConnection::new(
                Source::default(),
                0,
                0,
                0,
                0,
                false,
                String::default(),
                #[cfg(feature = "debugger")]
                String::default(),
                0,
            );
            let connection_to_1 = OutputConnection::new(
                Source::default(),
                1,
                0,
                0,
                0,
                false,
                String::default(),
                #[cfg(feature = "debugger")]
                String::default(),
                0,
            );

            let f_a = RuntimeFunction::new(
                #[cfg(feature = "debugger")]
                "fA",
                #[cfg(feature = "debugger")]
                "/fA",
                "file://fake/test",
                vec![Input::new(
                    #[cfg(feature = "debugger")] "",
                    &Some(Once(json!(1))))],
                0,
                0,
                &[
                    connection_to_0.clone(), // outputs to self:0
                    connection_to_1.clone(), // outputs to f_b:0
                ],
                false,
            );
            let f_b = test_function_b_not_init();
            let functions = vec![f_a, f_b]; // NOTE the order!
            let submission = Submission::new(
                &Url::parse("file:///temp/fake.toml").expect("Could not create Url"),
                1,
                #[cfg(feature = "debugger")]
                true,
            );
            let mut state = RunState::new(&functions, submission);
            #[cfg(feature = "metrics")]
            let mut metrics = Metrics::new(2);
            #[cfg(feature = "debugger")]
                let mut server = super::DummyServer{};
            #[cfg(feature = "debugger")]
                let mut debugger = super::dummy_debugger(&mut server);

            state.init();

            assert_eq!(state.get_function_state(0), State::Ready, "f_a should be Ready");
            assert_eq!(
                state.get_function_state(1),
                State::Waiting,
                "f_b should be in Waiting"
            );

            assert_eq!(
                state.next_job().expect("Couldn't get next job").function_id,
                0,
                "next() should return function_id=0 (f_a) for running"
            );

            // Event: fake running of function fA
            let result_job = Job {
                job_id: 0,
                function_id: 0,
                flow_id: 0,
                implementation: super::test_impl(),
                input_set: vec![json!(1)],
                result: Ok((Some(json!(1)), true)),
                connections: vec![connection_to_0, connection_to_1],
            };
            state.complete_job(
                #[cfg(feature = "metrics")]
                &mut metrics,
                &result_job,
                #[cfg(feature = "debugger")]
                &mut debugger,
            );

            // Test
            assert_eq!(state.get_function_state(1), State::Ready, "f_b should be Ready");
            assert_eq!(
                state.get_function_state(0),
                State::Blocked,
                "f_a should be Blocked on f_b"
            );

            let job = state.next_job().expect("Couldn't get next job");
            assert_eq!(
                job.function_id, 1,
                "next() should return function_id=1 (f_b) for running"
            );
            state.complete_job(
                #[cfg(feature = "metrics")]
                &mut metrics,
                &job,
                #[cfg(feature = "debugger")]
                &mut debugger,
            );

            let job = state.next_job().expect("Couldn't get next job");
            assert_eq!(
                job.function_id, 0,
                "next() should return function_id=0 (f_a) for running"
            );
            state.complete_job(
                #[cfg(feature = "metrics")]
                &mut metrics,
                &job,
                #[cfg(feature = "debugger")]
                &mut debugger,
            );
        }
    }

    /****************************** Miscellaneous tests **************************/
    mod functional_tests {
        use serde_json::json;
        // Tests using Debugger (and hence Client/Server connection) need to be executed in parallel
        // to avoid multiple trying to bind to the same socket at the same time
        use serial_test::serial;
        use url::Url;

        use flowcore::model::input::Input;
        #[cfg(feature = "metrics")]
        use flowcore::model::metrics::Metrics;
        use flowcore::model::output_connection::{OutputConnection, Source};
        use flowcore::model::runtime_function::RuntimeFunction;
        use flowcore::model::submission::Submission;

        use super::super::Job;
        use super::super::RunState;
        use super::super::State;

        fn test_functions() -> Vec<RuntimeFunction> {
            let out_conn1 = OutputConnection::new(
                Source::default(),
                1,
                0,
                0,
                0,
                false,
                String::default(),
                #[cfg(feature = "debugger")]
                String::default(),
                0,
            );
            let out_conn2 = OutputConnection::new(
                Source::default(),
                2,
                0,
                0,
                0,
                false,
                String::default(),
                #[cfg(feature = "debugger")]
                String::default(),
                0,
            );
            let p0 = RuntimeFunction::new(
                #[cfg(feature = "debugger")]
                "p0",
                #[cfg(feature = "debugger")]
                "/p0",
                "file://fake/test/p0",
                vec![], // input array
                0,
                0,
                &[out_conn1, out_conn2], // destinations
                false,
            ); // implementation
            let p1 = RuntimeFunction::new(
                #[cfg(feature = "debugger")]
                "p1",
                #[cfg(feature = "debugger")]
                "/p1",
                "file://fake/test/p1",
                vec![Input::new(
                    #[cfg(feature = "debugger")] "",
                    &None)], // inputs array
                1,
                0,
                &[],
                false,
            );
            let p2 = RuntimeFunction::new(
                #[cfg(feature = "debugger")]
                "p2",
                #[cfg(feature = "debugger")]
                "/p2",
                "file://fake/test/p2",
                vec![Input::new(
                    #[cfg(feature = "debugger")] "",
                    &None)], // inputs array
                2,
                0,
                &[],
                false,
            );
            vec![p0, p1, p2]
        }

        #[test]
        #[serial]
        fn blocked_works() {
            let submission = Submission::new(
                &Url::parse("file:///temp/fake.toml").expect("Could not create Url"),
                1,
                #[cfg(feature = "debugger")]
                true,
            );
            let mut state = RunState::new(&test_functions(), submission);
            #[cfg(feature = "debugger")]
                let mut server = super::DummyServer{};
            #[cfg(feature = "debugger")]
                let mut debugger = super::dummy_debugger(&mut server);

            // Indicate that 0 is blocked by 1 on input 0
            state.create_block(
                0,
                1,
                0,
                0,
                0,
                #[cfg(feature = "debugger")]
                &mut debugger,
            );
            assert!(state.blocked_sending(0));
        }

        #[test]
        fn get_works() {
            let submission = Submission::new(
                &Url::parse("file:///temp/fake.toml").expect("Could not create Url"),
                1,
                #[cfg(feature = "debugger")]
                true,
            );
            let state = RunState::new(&test_functions(), submission);
            let got = state.get_function(1);
            assert_eq!(got.id(), 1)
        }

        #[test]
        fn no_next_if_none_ready() {
            let submission = Submission::new(
                &Url::parse("file:///temp/fake.toml").expect("Could not create Url"),
                1,
                #[cfg(feature = "debugger")]
                true,
            );
            let mut state = RunState::new(&test_functions(), submission);

            assert!(state.next_job().is_none());
        }

        #[test]
        fn next_works() {
            let submission = Submission::new(
                &Url::parse("file:///temp/fake.toml").expect("Could not create Url"),
                1,
                #[cfg(feature = "debugger")]
                true,
            );
            let mut state = RunState::new(&test_functions(), submission);

            // Put 0 on the blocked/ready
            state.new_input_set(0, 0, true);

            assert_eq!(
                state.next_job().expect("Couldn't get next job").function_id,
                0
            );
        }

        #[test]
        fn inputs_ready_makes_ready() {
            let submission = Submission::new(
                &Url::parse("file:///temp/fake.toml").expect("Could not create Url"),
                1,
                #[cfg(feature = "debugger")]
                true,
            );
            let mut state = RunState::new(&test_functions(), submission);

            // Put 0 on the blocked/ready list depending on blocked status
            state.new_input_set(0, 0, true);

            assert_eq!(
                state.next_job().expect("Couldn't get next job").function_id,
                0
            );
        }

        #[test]
        #[serial]
        fn blocked_is_not_ready() {
            let submission = Submission::new(
                &Url::parse("file://temp/fake.toml").expect("Could not create Url"),
                1,
                #[cfg(feature = "debugger")]
                true,
            );
            let mut state = RunState::new(&test_functions(), submission);
            #[cfg(feature = "debugger")]
                let mut server = super::DummyServer{};
            #[cfg(feature = "debugger")]
                let mut debugger = super::dummy_debugger(&mut server);

            // Indicate that 0 is blocked by 1 on input 0
            state.create_block(
                0,
                1,
                0,
                0,
                0,
                #[cfg(feature = "debugger")]
                &mut debugger,
            );

            // Put 0 on the blocked/ready list depending on blocked status
            state.new_input_set(0, 0, true);

            assert!(state.next_job().is_none());
        }

        #[test]
        #[serial]
        fn unblocking_makes_ready() {
            let submission = Submission::new(
                &Url::parse("file:///temp/fake.toml").expect("Could not create Url"),
                1,
                #[cfg(feature = "debugger")]
                true,
            );
            let mut state = RunState::new(&test_functions(), submission);
            #[cfg(feature = "debugger")]
                let mut server = super::DummyServer{};
            #[cfg(feature = "debugger")]
                let mut debugger = super::dummy_debugger(&mut server);

            // Indicate that 0 is blocked by 1 and put 0 on the blocked list
            state.create_block(
                0,
                1,
                0,
                0,
                0,
                #[cfg(feature = "debugger")]
                &mut debugger,
            );
            // 0's inputs are now full, so it would be ready if it weren't blocked on output
            state.new_input_set(0, 0, true);
            // 0 does not show as ready.
            assert!(state.next_job().is_none());

            // now unblock senders to 1 (i.e. 0)
            state.unblock_senders(0, 1, 0);

            // Now function with id 0 should be ready and served up by next
            assert_eq!(
                state.next_job().expect("Couldn't get next job").function_id,
                0
            );
        }

        #[test]
        #[serial]
        fn unblocking_doubly_blocked_functions_not_ready() {
            let submission = Submission::new(
                &Url::parse("file:///temp/fake.toml").expect("Could not create Url"),
                1,
                #[cfg(feature = "debugger")]
                true,
            );
            let mut state = RunState::new(&test_functions(), submission);

            #[cfg(feature = "debugger")]
                let mut server = super::DummyServer{};
            #[cfg(feature = "debugger")]
                let mut debugger = super::dummy_debugger(&mut server);

            // Indicate that 0 is blocked by 1 and 2
            state.create_block(
                0,
                1,
                0,
                0,
                0,
                #[cfg(feature = "debugger")]
                &mut debugger,
            );
            state.create_block(
                0,
                2,
                0,
                0,
                0,
                #[cfg(feature = "debugger")]
                &mut debugger,
            );

            // Put 0 on the blocked/ready list depending on blocked status
            state.new_input_set(0, 0, true);

            assert!(state.next_job().is_none());

            // now unblock 0 by 1
            state.unblock_senders(0, 1, 0);

            // Now function with id 0 should still not be ready as still blocked on 2
            assert!(state.next_job().is_none());
        }

        #[test]
        fn wont_return_too_many_jobs() {
            let submission = Submission::new(
                &Url::parse("file:///temp/fake.toml").expect("Could not create Url"),
                1,
                #[cfg(feature = "debugger")]
                true,
            );
            let mut state = RunState::new(&test_functions(), submission);

            // Put 0 on the ready list
            state.new_input_set(0, 0, true);
            // Put 1 on the ready list
            state.new_input_set(1, 0, true);

            let job = state.next_job().expect("Couldn't get next job");
            assert_eq!(0, job.function_id);
            state.start(&job);

            assert!(state.next_job().is_none());
        }

        /*
            This test checks that a function with no output destinations (even if pure and produces
            some output) can be executed and nothing crashes
        */
        #[test]
        #[serial]
        fn pure_function_no_destinations() {
            let f_a = super::test_function_a_init();

            let functions = vec![f_a];
            let submission = Submission::new(
                &Url::parse("file:///temp/fake.toml").expect("Could not create Url"),
                1,
                #[cfg(feature = "debugger")]
                true,
            );
            let mut state = RunState::new(&functions, submission);
            #[cfg(feature = "metrics")]
            let mut metrics = Metrics::new(1);
            #[cfg(feature = "debugger")]
                let mut server = super::DummyServer{};
            #[cfg(feature = "debugger")]
                let mut debugger = super::dummy_debugger(&mut server);

            state.init();

            assert_eq!(
                state.next_job().expect("Couldn't get next job").function_id,
                0
            );

            // Event run f_a
            let job = Job {
                job_id: 0,
                function_id: 0,
                flow_id: 0,
                implementation: super::test_impl(),
                input_set: vec![json!(1)],
                result: Ok((Some(json!(1)), true)),
                connections: vec![],
            };

            // Test there is no problem producing an Output when no destinations to send it to
            state.complete_job(
                #[cfg(feature = "metrics")]
                &mut metrics,
                &job,
                #[cfg(feature = "debugger")]
                &mut debugger,
            );
            assert_eq!(State::Waiting, state.get_function_state(0), "f_a should be Waiting");
        }
    }

    mod misc {
        use serde_json::{json, Value};

        use flowcore::model::input::Input;
        use flowcore::model::output_connection::{OutputConnection, Source};
        use flowcore::model::runtime_function::RuntimeFunction;

        use super::super::RunState;

        #[test]
        fn test_array_order_0() {
            let value = json!(1);
            assert_eq!(RunState::array_order(&value), 0);
        }

        #[test]
        fn test_array_order_1_empty_array() {
            let value = json!([]);
            assert_eq!(RunState::array_order(&value), 1);
        }

        #[test]
        fn test_array_order_1() {
            let value = json!([1, 2, 3]);
            assert_eq!(RunState::array_order(&value), 1);
        }

        #[test]
        fn test_array_order_2() {
            let value = json!([[1, 2, 3], [2, 3, 4]]);
            assert_eq!(RunState::array_order(&value), 2);
        }

        fn test_function() -> RuntimeFunction {
            RuntimeFunction::new(
                #[cfg(feature = "debugger")]
                "test",
                #[cfg(feature = "debugger")]
                "/test",
                "file://fake/test",
                vec![Input::new(
                    #[cfg(feature = "debugger")] "",
                    &None)],
                0,
                0,
                &[],
                false,
            )
        }

        // Test type conversion and sending
        //                         |                   Destination
        //                         |Generic     Non-Array       Array       Array of Arrays
        // Value       Value order |    N/A         0               1       2      <---- Array Order
        //  Non-Array       (0)    |   send     (0) send        (-1) wrap   (-2) wrap in array of arrays
        //  Array           (1)    |   send     (1) iter        (0) send    (-1) wrap in array
        //  Array of Arrays (2)    |   send     (2) iter/iter   (1) iter    (0) send
        #[test]
        fn test_sending() {
            #[derive(Debug)]
            struct TestCase {
                value: Value,
                destination_is_generic: bool,
                destination_array_order: i32,
                value_expected: Value,
            }

            let test_cases = vec![
                // Column 0 test cases
                TestCase {
                    value: json!(1),
                    destination_is_generic: true,
                    destination_array_order: 0,
                    value_expected: json!(1),
                },
                TestCase {
                    value: json!([1]),
                    destination_is_generic: true,
                    destination_array_order: 0,
                    value_expected: json!([1]),
                },
                TestCase {
                    value: json!([[1, 2], [3, 4]]),
                    destination_is_generic: true,
                    destination_array_order: 0,
                    value_expected: json!([[1, 2], [3, 4]]),
                },
                // Column 1 Test Cases
                TestCase {
                    value: json!(1),
                    destination_is_generic: false,
                    destination_array_order: 0,
                    value_expected: json!(1),
                },
                TestCase {
                    value: json!([1, 2]),
                    destination_is_generic: false,
                    destination_array_order: 0,
                    value_expected: json!(1),
                },
                TestCase {
                    value: json!([[1, 2], [3, 4]]),
                    destination_is_generic: false,
                    destination_array_order: 0,
                    value_expected: json!(1),
                },
                // Column 2 Test Cases
                TestCase {
                    value: json!(1),
                    destination_is_generic: false,
                    destination_array_order: 1,
                    value_expected: json!([1]),
                },
                TestCase {
                    value: json!([1, 2]),
                    destination_is_generic: false,
                    destination_array_order: 1,
                    value_expected: json!([1, 2]),
                },
                TestCase {
                    value: json!([[1, 2], [3, 4]]),
                    destination_is_generic: false,
                    destination_array_order: 1,
                    value_expected: json!([1, 2]),
                },
                // Column 3 Test Cases
                TestCase {
                    value: json!(1),
                    destination_is_generic: false,
                    destination_array_order: 2,
                    value_expected: json!([[1]]),
                },
                TestCase {
                    value: json!([1, 2]),
                    destination_is_generic: false,
                    destination_array_order: 2,
                    value_expected: json!([[1, 2]]),
                },
                TestCase {
                    value: json!([[1, 2], [3, 4]]),
                    destination_is_generic: false,
                    destination_array_order: 2,
                    value_expected: json!([[1, 2], [3, 4]]),
                },
            ];

            for test_case in test_cases {
                // Setup
                let mut function = test_function();
                let destination = OutputConnection::new(
                    Source::default(),
                    0,
                    0,
                    0,
                    test_case.destination_array_order,
                    test_case.destination_is_generic,
                    String::default(),
                    #[cfg(feature = "debugger")]
                    String::default(),
                    0,
                );

                // Test
                assert!(RunState::type_convert_and_send(&mut function,
                    &destination, &test_case.value));

                // Check
                assert_eq!(
                    test_case.value_expected,
                    function
                        .take_input_set()
                        .expect("Couldn't get input set")
                        .remove(0)
                );
            }
        }
    }
}

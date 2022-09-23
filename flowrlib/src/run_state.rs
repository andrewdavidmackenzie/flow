use std::collections::{HashMap, HashSet};
use std::collections::hash_map::Entry;
use std::collections::VecDeque;
use std::fmt;

use log::{debug, error, info, trace};
use multimap::MultiMap;
use rand::Rng;
use serde_derive::{Deserialize, Serialize};
use serde_json::Value;

use flowcore::errors::*;
#[cfg(feature = "metrics")]
use flowcore::model::metrics::Metrics;
use flowcore::model::output_connection::OutputConnection;
use flowcore::model::output_connection::Source::{Input, Output};
use flowcore::model::runtime_function::RuntimeFunction;
use flowcore::model::submission::Submission;

use crate::block::Block;
#[cfg(debug_assertions)]
use crate::checks;
#[cfg(feature = "debugger")]
use crate::debugger::Debugger;
use crate::job::Job;

/// `State` represents the possible states it is possible for a function to be in
#[cfg(any(debug_assertions, feature = "debugger", test))]
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
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

/// Execution of of functions in the ready queue can be performed in different orders, using
/// different strategies to select the next function to execute.
#[derive(Deserialize, Serialize, Clone, Default)]
enum ExecutionStrategy {
    // Execute functions in the same order they complete and are put onto the queue
    InOrder,
    #[default]
    // Execute ready functions in a random order. Used to check semantics are independent of order
    Random,
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
/// Blocks on other senders due to Always Initializers and Loops
/// ==============================================================
/// After a function runs, its Always Initializers are used to refill inputs, and outputs (possibly to itself) are
/// sent, before determining that other functions sending to it should unblocked.
/// The initializers and loops to it's inputs have priority and the input(s) will be refilled
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
    /// blocks: - a list of blocks between functions
    blocks: HashSet<Block>,
    /// ready: Vec<function_id> - a list of functions by id that are ready to run
    ready: VecDeque<usize>,
    /// running: MultiMap<function_id, job_id> - a list of functions and jobs ids that are running
    running: MultiMap<usize, usize>,
    /// maintain a count of the number of jobs running
    num_running: usize,
    /// completed: functions that have run to completion and won't run again
    completed: HashSet<usize>,
    /// number of jobs sent for execution to date
    number_of_jobs_created: usize,
    /// Track which flow-function combinations are considered "busy" <flow_id, function_id>
    busy_flows: MultiMap<usize, usize>,
    /// Track which functions have finished and can be unblocked when flow goes not "busy"
    /// HashMap< <flow_id>, (function_id, vector of refilled io numbers of that function)>
    flow_blocks: HashMap<usize, HashSet<usize>>,
    /// The `Submission` that lead to this `RunState` object being created
    pub(crate) submission: Submission,
    /// The execution strategy being used
    strategy: ExecutionStrategy,
}

impl RunState {
    /// Create a new `RunState` struct from the list of functions provided and the `Submission`
    /// that was sent to be executed
    pub fn new(functions: Vec<RuntimeFunction>, submission: Submission) -> Self {
        RunState {
            functions,
            submission,
            blocked: HashSet::<usize>::new(),
            blocks: HashSet::<Block>::new(),
            ready: VecDeque::<usize>::new(),
            running: MultiMap::<usize, usize>::new(),
            num_running: 0,
            completed: HashSet::<usize>::new(),
            number_of_jobs_created: 0,
            busy_flows: MultiMap::<usize, usize>::new(),
            flow_blocks: HashMap::<usize, HashSet<usize>>::new(),
            strategy: ExecutionStrategy::Random,
        }
    }

    /// Get a reference to the vector of all functions
    pub(crate) fn get_functions(&self) -> &Vec<RuntimeFunction> {
        &self.functions
    }

    // Reset all values back to initial ones to enable debugging to restart from the initial state
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
        self.num_running = 0;
        self.completed.clear();
        self.number_of_jobs_created = 0;
        self.busy_flows.clear();
        self.flow_blocks.clear();
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
    pub(crate) fn init(&mut self) {
        #[cfg(feature = "debugger")]
        self.reset();

        debug!("Initializing all functions");
        for function in &mut self.functions {
            function.init();
            if function.can_run() {
                trace!("\t\t\tFunction #{} State set to 'Ready'", function.id());
                self.ready.push_back(function.id());
                self.busy_flows.insert(function.get_flow_id(), function.id());
            }
        }
    }

    /// Return the states a function is in
    #[cfg(any(debug_assertions, feature = "debugger", test))]
    pub fn get_function_states(&self, function_id: usize) -> Vec<State> {
        let mut states = vec![];

        if self.completed.contains(&function_id) {
            states.push(State::Completed);
        }

        if self.ready.contains(&function_id) {
            states.push(State::Ready);
        }

        if self.blocked.contains(&function_id) {
            states.push(State::Blocked);
        }

        if self.running.contains_key(&function_id) {
            states.push(State::Running);
        }

        if states.is_empty() {
            states.push(State::Waiting);
        }

        states
    }

    // See if the function is in only the specified state
    #[cfg(test)]
    fn function_state_is_only(&self, function_id: usize, state: State) -> bool {
        let function_states = self.get_function_states(function_id);
        function_states.len() == 1 && function_states.contains(&state)
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
    pub fn get_function(&self, id: usize) -> Option<&RuntimeFunction> {
        self.functions.get(id)
    }

    // Get a mutable reference to the function with `id`
    fn get_mut(&mut self, id: usize) -> Option<&mut RuntimeFunction> {
        self.functions.get_mut(id)
    }

    /// Get the HashSet of blocked function ids
    #[cfg(any(debug_assertions, feature = "debugger"))]
    pub fn get_blocks(&self) -> &HashSet<Block> {
        &self.blocks
    }

    #[cfg(debug_assertions)]
    /// Return the list of busy flows and what functions in each flow are busy
    pub fn get_busy_flows(&self) -> &MultiMap<usize, usize> {
        &self.busy_flows
    }

    #[cfg(debug_assertions)]
    /// Return the list of pending unblocks
    pub fn get_flow_blocks(&self) -> &HashMap<usize, HashSet<usize>> {
        &self.flow_blocks
    }

    // Return a new job to run, if there is one and there are not too many jobs already running
    pub(crate) fn new_job(&mut self) -> Option<Job> {
        if let Some(limit) = self.submission.max_parallel_jobs {
            if self.number_jobs_running() >= limit {
                trace!("Max Pending Job count of {limit} reached, skipping new jobs");
                return None;
            }
        }

        if self.ready.is_empty() {
            return None;
        }

        let function_id = match self.strategy {
            ExecutionStrategy::InOrder => self.ready.remove(0)?,
            ExecutionStrategy::Random => {
                // Generate random index in the range [0, len()-1]
                let index = rand::thread_rng().gen_range(0..self.ready.len());
                self.ready.remove(index)?
            },
        };

        self.create_job(function_id).map(|job| {
            self.running.insert(job.function_id, job.job_id);
            self.num_running += 1;
            let _ = self.block_external_flow_senders(job.job_id, job.function_id, job.flow_id);
            job
        })
    }

    // The function with id `blocker_function_id` in the flow with id `blocked_flow_id` has had a
    // job created from it's input so is a candidate to send more Values to from other functions that
    // previously were blocked sending to it.
    //
    // But we don't want to unblock them to send to it, until all other functions inside the same
    // flow are idle, and hence the flow becomes idle.
    fn block_external_flow_senders(
        &mut self,
        job_id: usize,
        blocker_function_id: usize,
        blocker_flow_id: usize,
    ) -> Result<()> {
        // Add this function to the pending unblock list for later when flow goes idle and senders
        // to it from *outside* this flow can be allowed to send to it.
        // The entry key is the blocker_flow_id and the entry all blocker_function_ids in that flow
        // that are pending to have senders to them unblocked
        match self.flow_blocks.entry(blocker_flow_id) {
            Entry::Occupied(mut o) => {
                // Add the `blocker_function_id` to the list of function in `blocker_flow_id` that
                // should be free to send to, once the flow eventually goes idle
                o.get_mut().insert(blocker_function_id);
            },
            Entry::Vacant(v) => {
                let mut new_set = HashSet::new();
                // Create a new list of function in `blocker_flow_id` that
                // should be free to send to, once the flow eventually goes idle
                // and add the `blocker_function_id` to it
                new_set.insert(blocker_function_id);
                // Add the entry for `blocker_flow_id` for when it goes idle later, to flow_blocks
                v.insert(new_set);
            }
        }
        trace!("Job #{job_id}:\t\tAdded a flow_block -> #{blocker_function_id}({blocker_flow_id})");

        Ok(())
    }

    /// get the number of jobs created to date in the flow's execution
    #[cfg(any(feature = "metrics", feature = "debugger"))]
    pub fn get_number_of_jobs_created(&self) -> usize {
        self.number_of_jobs_created
    }

    // Given a function id, prepare a job for execution that contains the input values, the
    // implementation and the destination functions the output should be sent to when done
    fn create_job(&mut self, function_id: usize) -> Option<Job> {
        self.number_of_jobs_created += 1;
        let job_id = self.number_of_jobs_created;

        let function = self.get_mut(function_id)?;

        match function.take_input_set() {
            Ok(input_set) => {
                let flow_id = function.get_flow_id();

                trace!("Job #{job_id}: NEW Job Created for Function #{function_id}({flow_id})");

                Some(Job {
                    job_id,
                    function_id,
                    flow_id,
                    implementation_location: function.get_implementation_location().to_string(),
                    input_set,
                    connections: function.get_output_connections().clone(),
                    result: Ok((None, false)),
                })
            }
            Err(e) => {
                error!(
                    "Job #{job_id}: Error '{e}' while creating job for Function #{function_id}"
                );
                None
            }
        }
    }

    // Complete a Job by taking its output and updating the run-list accordingly.
    //
    // If other functions were blocked trying to send to this one - we can now unblock them
    // as it has consumed it's inputs and they are free to be sent to again.
    //
    // Then take the output and send it to all destination IOs on different function it should be
    // sent to, marking the source function as blocked because those others must consume the output
    // if those other function have all their inputs, then mark them accordingly.
    #[allow(unused_variables, unused_assignments, unused_mut)]
    pub(crate) fn retire_job(
        &mut self,
        #[cfg(feature = "metrics")] metrics: &mut Metrics,
        job: &Job,
        #[cfg(feature = "debugger")] debugger: &mut Debugger,
    ) -> Result<(bool, bool)>{
        let mut display_next_output = false;
        let mut restart = false;

        self.running.retain(|&_, &job_id| job_id != job.job_id);
        self.num_running -= 1;

        match &job.result {
            Ok((output_value, function_can_run_again)) => {
                #[cfg(feature = "debugger")]
                debug!("Job #{}: Function #{} '{}' {:?} -> {:?}", job.job_id, job.function_id,
                        self.get_function(job.function_id).ok_or("No such function")?.name(),
                    job.input_set,  output_value);
                #[cfg(not(feature = "debugger"))]
                debug!("Job #{}: Function #{} {:?} -> {:?}", job.job_id, job.function_id,
                        job.input_set,  output_value);

                for connection in &job.connections {
                    let value_to_send = match &connection.source {
                        Output(route) => {
                            match output_value {
                                Some(output_v) => output_v.pointer(route),
                                None => None
                            }
                        },
                        Input(index) => job.input_set.get(*index),
                    };

                    if let Some(value) = value_to_send {
                        (display_next_output, restart) =
                            self.send_a_value(
                                job.function_id,
                                job.flow_id,
                                connection,
                                value.clone(),
                                #[cfg(feature = "metrics")] metrics,
                                #[cfg(feature = "debugger")] debugger,
                        )?;
                    } else {
                        trace!(
                            "Job #{}:\t\tNo value found at '{}'",
                            job.job_id, &connection.source
                        );
                    }
                }

                if *function_can_run_again {
                    let function = self.get_mut(job.function_id)
                        .ok_or("No such function")?;

                    // Refill any inputs with function initializers
                    function.init_inputs(false, false);

                    // NOTE: May have input sets due to sending to self via a loopback
                    if function.can_run() {
                        self.make_ready(job.function_id, job.flow_id);
                    }
                } else {
                    // otherwise mark it as completed as it will never run again
                    self.mark_as_completed(job.function_id);
                }
            },
            Err(e) => error!("Error in Job#{}: {e}", job.job_id)
        }

        // unblock any senders from other flows that can now run due to this function completing
        // causing the flow to be idle now
        (display_next_output, restart) = self.unblock_flows(job,
                               #[cfg(feature = "debugger")] debugger,
            )?;

        #[cfg(debug_assertions)]
        checks::check_invariants(self, job.job_id)?;

        trace!("Job #{}: Completed-----------------------", job.job_id);

        Ok((display_next_output, restart))
    }

    // Send a value produced as part of an output of running a job to a destination function on
    // a specific input, update the metrics and potentially enter the debugger
    fn send_a_value(
        &mut self,
        source_id: usize,
        source_flow_id: usize,
        connection: &OutputConnection,
        output_value: Value,
        #[cfg(feature = "metrics")] metrics: &mut Metrics,
        #[cfg(feature = "debugger")] debugger: &mut Debugger,
    ) -> Result<(bool, bool)> {
        let mut display_next_output = false;
        let mut restart = false;

        let route_str = match &connection.source {
            Output(route) if route.is_empty() => "".into(),
            Output(route) => format!(" from output route '{}'", route),
            Input(index) => format!(" from Job value at input #{}", index),
        };

        let loopback = source_id == connection.destination_id;
        let same_flow = source_flow_id == connection.destination_flow_id;

        if loopback {
            info!("\t\tFunction #{source_id} loopback of '{output_value}'{route_str} to Self:{}",
                    connection.destination_io_number);
        } else {
            info!("\t\tFunction #{source_id} sending '{output_value}'{route_str} to Function #{}:{}",
                    connection.destination_id, connection.destination_io_number);
        };

        #[cfg(feature = "debugger")]
        if let Output(route) = &connection.source {
            (display_next_output, restart) = debugger.check_prior_to_send(
                self,
                source_id,
                route,
                &output_value,
                connection.destination_id,
                connection.destination_io_number,
            )?;
        }

        let function = self.get_mut(connection.destination_id)
            .ok_or("Could not get function")?;
        let count_before = function.input_set_count();
        function.send(connection.destination_io_number, output_value);

        #[cfg(feature = "metrics")]
        metrics.increment_outputs_sent(); // not distinguishing array serialization / wrapping etc

        // Avoid a function blocking on itself when sending itself a value via a loopback and avoid
        // blocking sending internally within a flow
        let block = (function.input_count(connection.destination_io_number) > 0)
            && !loopback && !same_flow;
        let new_input_set_available = function.input_set_count() > count_before;

        if block {
            // TODO pass in connection
            (display_next_output, restart) = self.create_block(
                connection.destination_flow_id,
                connection.destination_id,
                connection.destination_io_number,
                source_id,
                source_flow_id,
                #[cfg(feature = "debugger")]
                    debugger,
            )?;
        }

        // postpone the decision about making the sending function Ready when we have a loopback
        // connection that sends a value to itself, as it may also send to other functions and need
        // to be blocked. But for all other receivers of values, make them Ready or Blocked
        if new_input_set_available && !loopback {
            self.make_ready_or_blocked(connection.destination_id, connection.destination_flow_id);
        }

        Ok((display_next_output, restart))
    }

    /// Get the set of (blocking_function_id, function's IO number causing the block)
    /// of blockers for a specific function of `id`
    #[cfg(any(feature = "debugger", debug_assertions))]
    pub fn get_output_blockers(&self, id: usize) -> Vec<usize> {
        let mut blockers = vec![];

        for block in &self.blocks {
            if block.blocked_function_id == id {
                blockers.push(block.blocking_function_id);
            }
        }

        blockers
    }

    // See if there is any block where the blocked function is the one we're looking for
    pub(crate) fn block_exists(&self, id: usize) -> bool {
        for block in &self.blocks {
            if block.blocked_function_id == id {
                return true;
            }
        }
        false
    }

    /// Return how many jobs are currently running
    pub fn number_jobs_running(&self) -> usize {
        self.num_running
    }

    /// Return how many jobs are ready to be run, but not running yet
    pub fn number_jobs_ready(&self) -> usize {
        self.ready.len()
    }

    /// An input blocker is another function that is the only function connected to an empty input
    /// of target function, and which is not ready to run, hence target function cannot run.
    #[cfg(feature = "debugger")]
    pub fn get_input_blockers(&self, target_id: usize) -> Result<Vec<usize>> {
        let mut input_blockers = vec![];
        let target_function = self.get_function(target_id)
            .ok_or("No such function")?;

        // for each empty input of the target function
        for (target_io, input) in target_function.inputs().iter().enumerate() {
            if input.count() == 0 {
                let mut senders = Vec::<usize>::new();

                // go through all functions to see if sends to the target function on this input
                for sender_function in &self.functions {
                    // if the sender function is not ready to run
                    if !self.ready.contains(&sender_function.id()) {
                        // for each output route of sending function, see if it is sending to the target function and input
                        //(ref _output_route, destination_id, io_number, _destination_path)
                        for destination in sender_function.get_output_connections() {
                            if (destination.destination_id == target_id)
                                && (destination.destination_io_number == target_io)
                            {
                                senders.push(sender_function.id());
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

        Ok(input_blockers)
    }

    // A function may be able to produce output, either because:
    // - it has a full set of inputs, so can be run and produce an output
    // - it has no input and is impure, so can run and produce an output
    // In which case it should transition to one of two states: Ready or Blocked
    pub(crate) fn make_ready_or_blocked(&mut self, function_id: usize, flow_id: usize) {
        if self.block_exists(function_id) {
            trace!( "\t\t\tFunction #{function_id} blocked on output. State set to 'Blocked'");
            self.blocked.insert(function_id);
        } else {
            self.make_ready(function_id, flow_id);
        }
    }

    fn make_ready(&mut self, function_id: usize, flow_id: usize) {
        trace!("\t\t\tFunction #{function_id} State set to 'Ready'");
        self.ready.push_back(function_id);
        self.busy_flows.insert(flow_id, function_id);
    }

    /// Return how many functions exist in this flow being executed
    #[cfg(any(feature = "debugger", feature = "metrics"))]
    pub fn num_functions(&self) -> usize {
        self.functions.len()
    }

    // Remove blocks on functions sending to another function inside the `blocker_flow_id` flow
    // if that has just gone idle
    #[allow(unused_variables, unused_assignments, unused_mut)]
    fn unblock_flows(&mut self,
                     job: &Job,
                     #[cfg(feature = "debugger")] debugger: &mut Debugger,
        ) -> Result<(bool, bool)> {
        let mut display_next_output = false;
        let mut restart = false;

        self.remove_from_busy(job.function_id);

        // if flow is now idle, remove any blocks on sending to functions in the flow
        if self.busy_flows.get(&job.flow_id).is_none() {
            debug!("Job #{}:\tFlow #{} is now idle, so removing blocks on external functions to it",
                job.job_id, job.flow_id);

            #[cfg(feature = "debugger")]
            {
                (display_next_output, restart) = debugger.check_prior_to_flow_unblock(self,
                                                                                      job.flow_id)?;
            }

            if let Some(blocker_functions) = self.flow_blocks.remove(&job.flow_id) {
                for blocker_function_id in blocker_functions {
                    self.remove_blocks(blocker_function_id)?;
                }
            }

            // run flow initializers on functions in the flow that has just gone idle
            self.run_flow_initializers(job.flow_id);
        }

        Ok((display_next_output, restart))
    }

    // Remove ONE entry of <flow_id, function_id> from the busy_flows multi-map
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

    // Remove blocks on functions blocked sending to `blocker_function_id`
    // If a sending function has no remaining blocks preventing it from sending then unblock that function
    fn remove_blocks(&mut self, blocker_function_id: usize) -> Result<()>
    {
        let mut blocks_to_remove = vec![];

        // Remove matching blocks and maintain a list of sender functions to unblock
        for block in &self.blocks {
            if block.blocking_function_id == blocker_function_id {
                blocks_to_remove.push(block.clone());
            }
        }

        // Remove blocks between the sender and the destination. Note that a sender can send to
        // multiple destinations and so could still be blocked sending to other functions
        for block in blocks_to_remove {
            self.blocks.remove(&block);
            trace!("\t\t\tBlock removed {:?}", block);

            if self.blocked.contains(&block.blocked_function_id) && !self.block_exists(block.blocked_function_id) {
                trace!("\t\t\t\tFunction #{} removed from 'blocked' list", block.blocked_function_id);
                self.blocked.remove(&block.blocked_function_id);

                let function = self.get_function(block.blocked_function_id).ok_or("No such function")?;
                if function.can_run() {
                    self.make_ready_or_blocked(block.blocked_function_id, block.blocked_flow_id);
                }
            }
        }

        Ok(())
    }

    fn run_flow_initializers(&mut self, flow_id: usize) {
        let mut initialized_functions = Vec::<usize>::new();
        for function in &mut self.functions {
            if function.get_flow_id() == flow_id {
                let could_run_before = function.can_run();
                function.init_inputs(false, true);
                let can_run_now = function.can_run();

                if can_run_now && !could_run_before {
                    initialized_functions.push(function.id());
                }
            }
        }

        for function_id in initialized_functions {
            self.make_ready_or_blocked(function_id, flow_id);
        }
    }

    // Mark a function (via its ID) as having run to completion
    pub(crate) fn mark_as_completed(&mut self, function_id: usize) {
        self.completed.insert(function_id);
    }

    // Create a 'block" indicating that function `blocked_function_id` cannot run as it has sends
    // to an input on function 'blocking_function_id' that is already full.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn create_block(
        &mut self,
        blocking_flow_id: usize,
        blocking_function_id: usize,
        blocking_io_number: usize,
        blocked_function_id: usize,
        blocked_flow_id: usize,
        #[cfg(feature = "debugger")] debugger: &mut Debugger,
    ) -> Result<(bool, bool)>{
        let block = Block::new(
            blocking_flow_id,
            blocking_function_id,
            blocking_io_number,
            blocked_function_id,
            blocked_flow_id,
        );

        trace!("\t\t\t\t\tCreating Block {:?}", block);
        self.blocks.insert(block.clone());
        #[cfg(feature = "debugger")]
        return debugger.check_on_block_creation(self, &block);
        #[cfg(not(feature = "debugger"))]
        Ok((false, false))
    }
}

impl fmt::Display for RunState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}\n", self.submission)?;

        writeln!(f, "RunState:")?;
        writeln!(f, "       Jobs Created: {}", self.number_of_jobs_created)?;
        writeln!(f, "       Jobs Running: {}", self.num_running)?;
        writeln!(f, ". Functions Blocked: {:?}", self.blocked)?;
        writeln!(f, "             Blocks: {:?}", self.blocks)?;
        writeln!(f, "    Functions Ready: {:?}", self.ready)?;
        writeln!(f, "  Functions Running: {:?}", self.running)?;
        writeln!(f, "Functions Completed: {:?}", self.completed)?;
        writeln!(f, "         Flows Busy: {:?}", self.busy_flows)?;
        write!(f, "     Pending Unblocks: {:?}", self.flow_blocks)
    }
}

#[cfg(test)]
mod test {
    use serde_json::json;
    use serde_json::Value;

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
    use crate::run_state::State;
    #[cfg(feature = "debugger")]
    use crate::server::DebuggerProtocol;

    use super::Job;
    use super::RunState;

    fn test_function_a_to_b_not_init() -> RuntimeFunction {
        let connection_to_f1 = OutputConnection::new(
            Source::default(),
            1,
            0,
            0,
            "/fB".to_string(),
            #[cfg(feature = "debugger")]
            String::default(),
        );

        RuntimeFunction::new(
            #[cfg(feature = "debugger")]
            "fA",
            #[cfg(feature = "debugger")]
            "/fA",
            "file://fake/test",
            vec![Input::new(
                        #[cfg(feature = "debugger")] "", 0, false,
                            None, None)],
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
            "/fB".to_string(),
            #[cfg(feature = "debugger")]
            String::default(),
        );
        RuntimeFunction::new(
            #[cfg(feature = "debugger")]
            "fA",
            #[cfg(feature = "debugger")]
            "/fA",
            "file://fake/test",
            vec![Input::new(
                            #[cfg(feature = "debugger")] "", 0, false,
                            Some(Once(json!(1))), None)],
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
                #[cfg(feature = "debugger")] "", 0, false,
                Some(Once(json!(1))), None)],
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
                #[cfg(feature = "debugger")] "", 0, false,
                None, None)],
            1,
            0,
            &[],
            false,
        )
    }

    fn test_job(state: &mut RunState, source_function_id: usize, destination_function_id: usize) -> Job {
        let out_conn = OutputConnection::new(
            Source::default(),
            destination_function_id,
            0,
            0,
            String::default(),
            #[cfg(feature = "debugger")]
            String::default(),
        );
        state.num_running += 1;
        Job {
            job_id: 1,
            function_id: source_function_id,
            flow_id: 0,
            implementation_location: "test".into(),
            input_set: vec![json!(1)],
            result: Ok((Some(json!(1)), true)),
            connections: vec![out_conn],
        }
    }

    #[cfg(feature = "debugger")]
    struct DummyServer;
    #[cfg(feature = "debugger")]
    impl DebuggerProtocol for DummyServer {
        fn start(&mut self) {}
        fn job_breakpoint(&mut self, _job: &Job, _function: &RuntimeFunction, _states: Vec<State>) {}
        fn block_breakpoint(&mut self, _block: &Block) {}
        fn flow_unblock_breakpoint(&mut self, _flow_id: usize) {}
        fn send_breakpoint(&mut self, _: &str, _source_process_id: usize, _output_route: &str, _value: &Value,
                           _destination_id: usize, _destination_name:&str, _input_name: &str, _input_number: usize) {}
        fn job_error(&mut self, _job: &Job) {}
        fn job_completed(&mut self, _job: &Job) {}
        fn blocks(&mut self, _blocks: Vec<Block>) {}
        fn outputs(&mut self, _output: Vec<OutputConnection>) {}
        fn input(&mut self, _input: Input) {}
        fn function_list(&mut self, _functions: &[RuntimeFunction]) {}
        fn function_states(&mut self, _function: RuntimeFunction, _function_states: Vec<State>) {}
        fn run_state(&mut self, _run_state: &RunState) {}
        fn message(&mut self, _message: String) {}
        fn panic(&mut self, _state: &RunState, _error_message: String) {}
        fn debugger_exiting(&mut self) {}
        fn debugger_resetting(&mut self) {}
        fn debugger_error(&mut self, _error: String) {}
        fn execution_starting(&mut self) {}
        fn execution_ended(&mut self) {}
        fn get_command(&mut self, _state: &RunState) -> Result<DebugCommand> {
            unimplemented!();
        }
    }

    #[cfg(feature = "debugger")]
    fn dummy_debugger(server: &mut dyn DebuggerProtocol) -> Debugger {
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
                None,
                #[cfg(feature = "debugger")]
                true,
            );
            let mut state = RunState::new(functions, submission);
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
                None,
                #[cfg(feature = "debugger")]
                true,
            );
            let mut state = RunState::new(vec![], submission);
            state.init();
            assert_eq!(0, state.get_number_of_jobs_created(), "At init jobs() should be 0");
            assert_eq!(0, state.number_jobs_ready());
        }

        #[cfg(feature = "debugger")]
        #[test]
        fn zero_blocks_at_init() {
            let submission = Submission::new(
                &Url::parse("file:///temp/fake.toml").expect("Could not create Url"),
                None,
                #[cfg(feature = "debugger")]
                true,
            );
            let mut state = RunState::new(vec![], submission);
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
                None,
                #[cfg(feature = "debugger")]
                true,
            );
            let mut state = RunState::new(vec![], submission);
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
                None,
                #[cfg(feature = "debugger")]
                true,
            );
            let mut state = RunState::new(vec![], submission);
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
        use flowcore::model::input::InputInitializer::Always;
        #[cfg(feature = "metrics")]
        use flowcore::model::metrics::Metrics;
        use flowcore::model::output_connection::{OutputConnection, Source};
        use flowcore::model::runtime_function::RuntimeFunction;
        use flowcore::model::submission::Submission;

        use crate::run_state::test::test_function_b_not_init;

        use super::super::Job;
        use super::super::RunState;
        use super::super::State;

        #[test]
        fn to_ready_1_on_init() {
            let f_a = super::test_function_a_to_b();
            let f_b = test_function_b_not_init();
            let functions = vec![f_a, f_b];
            let submission = Submission::new(
                &Url::parse("file:///temp/fake.toml").expect("Could not create Url"),
                None,
                #[cfg(feature = "debugger")]
                true,
            );
            let mut state = RunState::new(functions, submission);

            // Event
            state.init();

            // Test
            assert!(state.function_state_is_only(0, State::Ready), "f_a should be Ready");
            assert_eq!(1, state.number_jobs_ready());
            assert!(
                state.function_state_is_only(1, State::Waiting),
                "f_b should be waiting for input"
            );
        }

        #[test]
        fn input_blocker() {
            let f_a = super::test_function_a_to_b_not_init();
            let f_b = test_function_b_not_init();
            let functions = vec![f_a, f_b];
            let submission = Submission::new(
                &Url::parse("file:///temp/fake.toml").expect("Could not create Url"),
                None,
                #[cfg(feature = "debugger")]
                true,
            );
            let mut state = RunState::new(functions, submission);

            // Event
            state.init();

            // Test
            assert!(
                state.function_state_is_only(0, State::Waiting),
                "f_a should be waiting for input"
            );
            assert!(
                state.function_state_is_only(1, State::Waiting),
                "f_b should be waiting for input"
            );
            #[cfg(feature = "debugger")]
            assert!(
                state.get_input_blockers(1).expect("Could not get blockers").contains(&0),
                "f_b should be waiting for input from f_a"
            )
        }

        #[test]
        fn to_ready_2_on_init() {
            let f_a = super::test_function_a_to_b();
            let f_b = test_function_b_not_init();
            let functions = vec![f_a, f_b];
            let submission = Submission::new(
                &Url::parse("file:///temp/fake.toml").expect("Could not create Url"),
                None,
                #[cfg(feature = "debugger")]
                true,
            );
            let mut state = RunState::new(functions, submission);

            // Event
            state.init();

            // Test
            assert!(state.function_state_is_only(0, State::Ready), "f_a should be Ready");
        }

        #[test]
        fn to_ready_3_on_init() {
            let f_a = super::test_function_a_init();
            let functions = vec![f_a];
            let submission = Submission::new(
                &Url::parse("file:///temp/fake.toml").expect("Could not create Url"),
                None,
                #[cfg(feature = "debugger")]
                true,
            );
            let mut state = RunState::new(functions, submission);

            // Event
            state.init();

            // Test
            assert!(state.function_state_is_only(0, State::Ready), "f_a should be Ready");
        }

        fn test_function_a_not_init() -> RuntimeFunction {
            RuntimeFunction::new(
                #[cfg(feature = "debugger")]
                "fA",
                #[cfg(feature = "debugger")]
                "/fA",
                "file://fake/test",
                vec![Input::new(
                    #[cfg(feature = "debugger")] "", 0, false,
                    None, None)],
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
                None,
                #[cfg(feature = "debugger")]
                true,
            );
            let mut state = RunState::new(functions, submission);

            // Event
            state.init();

            // Test
            assert!(state.function_state_is_only(0, State::Waiting), "f_a should be Waiting");
        }

        #[test]
        fn ready_to_running_on_next() {
            let f_a = super::test_function_a_init();
            let functions = vec![f_a];
            let submission = Submission::new(
                &Url::parse("file:///temp/fake.toml").expect("Could not create Url"),
                None,
                #[cfg(feature = "debugger")]
                true,
            );
            let mut state = RunState::new(functions, submission);
            state.init();
            assert!(state.function_state_is_only(0, State::Ready), "f_a should be Ready");

            // Event
            let _ = state.new_job().expect("Couldn't get next job");

            // Test
            assert!(state.function_state_is_only(0, State::Running), "f_a should be Running");
        }

        #[test]
        fn unready_not_to_running_on_next() {
            let f_a = test_function_a_not_init();
            let functions = vec![f_a];
            let submission = Submission::new(
                &Url::parse("file:///temp/fake.toml").expect("Could not create Url"),
                None,
                #[cfg(feature = "debugger")]
                true,
            );
            let mut state = RunState::new(functions, submission);
            state.init();
            assert!(state.function_state_is_only(0, State::Waiting), "f_a should be Waiting");

            // Event
            assert!(state.new_job().is_none(), "next_job() should return None");

            // Test
            assert!(state.function_state_is_only(0, State::Waiting), "f_a should be Waiting");
        }

        fn test_job() -> Job {
            Job {
                job_id: 1,
                function_id: 0,
                flow_id: 0,
                implementation_location: "test".into(),
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
                    #[cfg(feature = "debugger")] "", 0, false,
                    Some(Always(json!(1))), None)],
                0,
                0,
                &[],
                false,
            );
            let functions = vec![f_a];
            let submission = Submission::new(
                &Url::parse("file:///temp/fake.toml").expect("Could not create Url"),
                None,
                #[cfg(feature = "debugger")]
                true,
            );
            let mut state = RunState::new(functions, submission);
            #[cfg(feature = "metrics")]
            let mut metrics = Metrics::new(1);
            #[cfg(feature = "debugger")]
                let mut server = super::DummyServer{};
            #[cfg(feature = "debugger")]
                let mut debugger = super::dummy_debugger(&mut server);

            state.init();
            assert!(state.function_state_is_only(0, State::Ready), "f_a should be Ready");
            let job = state.new_job().expect("Couldn't get next job");
            assert_eq!(0, job.function_id, "next() should return function_id = 0");

            assert!(state.function_state_is_only(0, State::Running), "f_a should be Running");

            // Event
            let job = test_job();
            let _ = state.retire_job(
                #[cfg(feature = "metrics")]
                &mut metrics,
                &job,
                #[cfg(feature = "debugger")]
                &mut debugger,
            );

            // Test
            assert!(
                state.function_state_is_only(0, State::Ready),
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
                None,
                #[cfg(feature = "debugger")]
                true,
            );
            let mut state = RunState::new(functions, submission);
            #[cfg(feature = "metrics")]
            let mut metrics = Metrics::new(1);
            #[cfg(feature = "debugger")]
                let mut server = super::DummyServer{};
            #[cfg(feature = "debugger")]
                let mut debugger = super::dummy_debugger(&mut server);

            state.init();
            assert!(state.function_state_is_only(0, State::Ready), "f_a should be Ready");
            let job = state.new_job().expect("Couldn't get next job");
            assert_eq!(0, job.function_id, "next() should return function_id = 0");

            assert!(state.function_state_is_only(0, State::Running), "f_a should be Running");

            // Event
            let job = test_job();
            let _ = state.retire_job(
                #[cfg(feature = "metrics")]
                &mut metrics,
                &job,
                #[cfg(feature = "debugger")]
                &mut debugger,
            );

            // Test
            assert!(state.function_state_is_only(0, State::Waiting),
                    "f_a should be Waiting again"
            );
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
                String::default(),
                #[cfg(feature = "debugger")]
                String::default(),
            );
            let f_b = RuntimeFunction::new(
                #[cfg(feature = "debugger")]
                "fB",
                #[cfg(feature = "debugger")]
                "/fB",
                "file://fake/test",
                vec![Input::new(
                    #[cfg(feature = "debugger")] "", 0, false,
                    None, None)],
                1,
                0,
                &[out_conn],
                false,
            );
            let functions = vec![f_a, f_b];
            let submission = Submission::new(
                &Url::parse("file:///temp/fake.toml").expect("Could not create Url"),
                None,
                #[cfg(feature = "debugger")]
                true,
            );
            let mut state = RunState::new(functions, submission);
            #[cfg(feature = "metrics")]
            let mut metrics = Metrics::new(1);
            #[cfg(feature = "debugger")]
                let mut server = super::DummyServer{};
            #[cfg(feature = "debugger")]
                let mut debugger = super::dummy_debugger(&mut server);

            state.init();
            assert!(state.function_state_is_only(0, State::Waiting), "f_a should be Waiting");

            // Event run f_b which will send to f_a
            let job = super::test_job(&mut state, 1, 0);

            let _ = state.retire_job(
                #[cfg(feature = "metrics")]
                &mut metrics,
                &job,
                #[cfg(feature = "debugger")]
                &mut debugger,
            );

            // Test
            assert!(state.function_state_is_only(0, State::Ready), "f_a should be Ready");
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
                String::default(),
                #[cfg(feature = "debugger")]
                String::default(),
            );
            let f_b = RuntimeFunction::new(
                #[cfg(feature = "debugger")]
                "fB",
                #[cfg(feature = "debugger")]
                "/fB",
                "file://fake/test",
                vec![Input::new(
                    #[cfg(feature = "debugger")] "", 0, false,
                    Some(Always(json!(1))), None)],
                1,
                0,
                &[connection_to_f0],
                false,
            );
            let functions = vec![f_a, f_b];
            let submission = Submission::new(
                &Url::parse("file:///temp/fake.toml").expect("Could not create Url"),
                None,
                #[cfg(feature = "debugger")]
                true,
            );
            let mut state = RunState::new(functions, submission);
            #[cfg(feature = "metrics")]
            let mut metrics = Metrics::new(1);
            #[cfg(feature = "debugger")]
                let mut server = super::DummyServer{};
            #[cfg(feature = "debugger")]
                let mut debugger = super::dummy_debugger(&mut server);

            state.init();

            assert!(state.function_state_is_only(1, State::Ready), "f_b should be Ready");
            assert!(
                state.function_state_is_only(0, State::Waiting),
                "f_a should be in Waiting"
            );

            // create output from f_b as if it had run - will send to f_a
            let job = super::test_job(&mut state, 1, 0);
            let _ = state.retire_job(
                #[cfg(feature = "metrics")]
                &mut metrics,
                &job,
                #[cfg(feature = "debugger")]
                &mut debugger,
            );

            // Test
            assert!(state.function_state_is_only(0, State::Ready), "f_a should be Ready");
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
                String::default(),
                #[cfg(feature = "debugger")]
                String::default(),
            );
            let out_conn2 = OutputConnection::new(
                Source::default(),
                2,
                0,
                0,
                String::default(),
                #[cfg(feature = "debugger")]
                String::default(),
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
                    #[cfg(feature = "debugger")] "", 0, false,
                    None, None)], // inputs array
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
                    #[cfg(feature = "debugger")] "", 0, false,
                    None, None)], // inputs array
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
                None,
                #[cfg(feature = "debugger")]
                true,
            );
            let mut state = RunState::new(test_functions(), submission);
            #[cfg(feature = "debugger")]
                let mut server = super::DummyServer{};
            #[cfg(feature = "debugger")]
                let mut debugger = super::dummy_debugger(&mut server);

            // Indicate that 0 is blocked by 1 on input 0
            let _ = state.create_block(
                0,
                1,
                0,
                0,
                0,
                #[cfg(feature = "debugger")]
                &mut debugger,
            );
            assert!(!state.get_output_blockers(0).is_empty());
        }

        #[test]
        fn get_works() {
            let submission = Submission::new(
                &Url::parse("file:///temp/fake.toml").expect("Could not create Url"),
                None,
                #[cfg(feature = "debugger")]
                true,
            );
            let state = RunState::new(test_functions(), submission);
            let got = state.get_function(1)
                .ok_or("Could not get function by id").expect("Could not get function with that id");
            assert_eq!(got.id(), 1)
        }

        #[test]
        fn no_next_if_none_ready() {
            let submission = Submission::new(
                &Url::parse("file:///temp/fake.toml").expect("Could not create Url"),
                None,
                #[cfg(feature = "debugger")]
                true,
            );
            let mut state = RunState::new(test_functions(), submission);

            assert!(state.new_job().is_none());
        }

        #[test]
        fn next_works() {
            let submission = Submission::new(
                &Url::parse("file:///temp/fake.toml").expect("Could not create Url"),
                None,
                #[cfg(feature = "debugger")]
                true,
            );
            let mut state = RunState::new(test_functions(), submission);

            // Put 0 on the blocked/ready
            state.make_ready_or_blocked(0, 0);

            state.new_job().expect("Couldn't get next job");
        }

        #[test]
        fn inputs_ready_makes_ready() {
            let submission = Submission::new(
                &Url::parse("file:///temp/fake.toml").expect("Could not create Url"),
                None,
                #[cfg(feature = "debugger")]
                true,
            );
            let mut state = RunState::new(test_functions(), submission);

            // Put 0 on the blocked/ready list depending on blocked status
            state.make_ready_or_blocked(0, 0);

            state.new_job().expect("Couldn't get next job");
        }

        #[test]
        #[serial]
        fn blocked_is_not_ready() {
            let submission = Submission::new(
                &Url::parse("file://temp/fake.toml").expect("Could not create Url"),
                None,
                #[cfg(feature = "debugger")]
                true,
            );
            let mut state = RunState::new(test_functions(), submission);
            #[cfg(feature = "debugger")]
                let mut server = super::DummyServer{};
            #[cfg(feature = "debugger")]
                let mut debugger = super::dummy_debugger(&mut server);

            // Indicate that 0 is blocked by 1 on input 0
            let _ = state.create_block(
                0,
                1,
                0,
                0,
                0,
                #[cfg(feature = "debugger")]
                &mut debugger,
            );

            // Put 0 on the blocked/ready list depending on blocked status
            state.make_ready_or_blocked(0, 0);

            assert!(state.new_job().is_none());
        }

        #[test]
        #[serial]
        fn unblocking_doubly_blocked_functions_not_ready() {
            let submission = Submission::new(
                &Url::parse("file:///temp/fake.toml").expect("Could not create Url"),
                None,
                #[cfg(feature = "debugger")]
                true,
            );
            let mut state = RunState::new(test_functions(), submission);

            #[cfg(feature = "debugger")]
                let mut server = super::DummyServer{};
            #[cfg(feature = "debugger")]
                let mut debugger = super::dummy_debugger(&mut server);

            // Indicate that 0 is blocked by 1 and 2
            let _ = state.create_block(
                0,
                1,
                0,
                0,
                0,
                #[cfg(feature = "debugger")]
                &mut debugger,
            );
            let _ = state.create_block(
                0,
                2,
                0,
                0,
                0,
                #[cfg(feature = "debugger")]
                &mut debugger,
            );

            // Put 0 on the blocked/ready list depending on blocked status
            state.make_ready_or_blocked(0, 0);

            assert!(state.new_job().is_none());

            // now unblock 0 by 1
            state.block_external_flow_senders(0, 1, 0)
                .expect("Could not unblock");

            // Now function with id 0 should still not be ready as still blocked on 2
            assert!(state.new_job().is_none());
        }

        #[test]
        #[serial]
        fn wont_return_too_many_jobs() {
            let submission = Submission::new(
                &Url::parse("file:///temp/fake.toml").expect("Could not create Url"),
                None,
                #[cfg(feature = "debugger")]
                true,
            );
            // test_functions has 3 functions
            let mut state = RunState::new(test_functions(), submission);

            state.init();

            let _ = state.new_job().expect("Couldn't get next job");

            assert!(state.new_job().is_none(), "Did not expect a Ready job!");
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
                None,
                #[cfg(feature = "debugger")]
                true,
            );
            let mut state = RunState::new(functions, submission);
            #[cfg(feature = "metrics")]
            let mut metrics = Metrics::new(1);
            #[cfg(feature = "debugger")]
                let mut server = super::DummyServer{};
            #[cfg(feature = "debugger")]
                let mut debugger = super::dummy_debugger(&mut server);

            state.init();

            state.new_job().expect("Couldn't get next job");

            // Event run f_a
            let job = Job {
                job_id: 0,
                function_id: 0,
                flow_id: 0,
                implementation_location: "test".into(),
                input_set: vec![json!(1)],
                result: Ok((Some(json!(1)), true)),
                connections: vec![],
            };

            // Test there is no problem producing an Output when no destinations to send it to
            let _ = state.retire_job(
                #[cfg(feature = "metrics")]
                &mut metrics,
                &job,
                #[cfg(feature = "debugger")]
                &mut debugger,
            );
            assert!(state.function_state_is_only(0, State::Running), "f_a should be Running");
        }
    }
}

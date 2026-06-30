use std::collections::VecDeque;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::time::Instant;

use log::{debug, error, info, trace, warn};
use serde_derive::{Deserialize, Serialize};
use serde_json::Value;

use flowcore::errors::Result;
use flowcore::model::input::InitReason;
#[cfg(feature = "metrics")]
use flowcore::model::metrics::Metrics;
use flowcore::model::output_connection::OutputConnection;
use flowcore::model::output_connection::Source::{Input, Output};
use flowcore::model::runtime_function::RuntimeFunction;
use flowcore::model::submission::Submission;
use flowcore::RunAgain;

#[cfg(debug_assertions)]
use crate::checks;
#[cfg(feature = "debugger")]
use crate::debugger::Debugger;
use crate::job::{Job, Payload};

/// `State` represents the possible states it is possible for a function to be in
#[cfg(any(debug_assertions, feature = "debugger", test))]
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum State {
    /// Ready     - Function will be in Ready state when all of its inputs are full
    Ready,
    /// Waiting   - Function is in the Waiting state when at least one of its inputs is not full
    Waiting,
    /// Running   - Function is in Running state when it has been picked from the Ready list for
    /// execution using the `next` function
    Running,
    /// Completed - Function has indicated that it no longer wants to be run, so it's execution
    ///           has completed.
    Completed,
}

/// `RunState` is a structure that maintains the state of all the functions in the currently
/// executing flow.
///
/// The Semantics of a Flow's `RunState`
/// ==================================
/// The semantics of the state of each function in a flow and the flow over are described here
/// and the tests of the struct attempt to reproduce and confirm as many of them as is possible
///
/// Terminology
/// ===========
/// * function        - an entry in the manifest and the flow graph that may take inputs, will
///   execute an implementation on a Job and may produce an Output
/// * input           - a function may have 0 or more inputs that accept values required for it's
///   execution
/// * implementation  - the code that is run, accepting 0 or more input values performing some
///   calculations and possibly producing an output value. One implementation can
///   be used by multiple functions in a flow
/// * destinations    - a set of other functions and their specific inputs that a function is
///   connected to and hence where the output value is sent when execution is
///   completed
/// * job             - a job is the bundle of information necessary to execute. It consists of the
///   function's id, the input values, the implementation to run, and the
///   destinations to send the output value to
/// * execution       - the act of running an implementation on the input values to produce an
///   output
/// * output          - a function when ran produces an output. The output contains the id of the
///   function that was ran, the input values (for debugging), the result
///   (optional value plus an indicator if the function wishes to be ran again
///   when ready), the destinations to send any value to and an optional error
///   string.
///
/// Start-up
/// ==============
/// At start-up all functions are initialized. For each of the functions inputs their
/// `init_inputs` function will be called, meaning that some inputs may be initialized (filled).
/// If all inputs are full then the Function will be ready to run.
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
/// If a pure function has an output but it is not used (not connected to any input) then the
/// function should have no affect on the execution of the flow and the optimizer may remove it and
/// all connections to its input. That in turn may affect other functions which can be removed,
/// until there are no more left to remove.
/// Thus at run-time, a pure function with it's output unused is not expected and no special
/// handling of that case is taken. If a manifest is read where a pure function has no destinations,
/// then it will be run (when it received inputs) and it's output discarded.
/// That is sub-optimal execution but no errors should result. Hence the role of the optimizer at
/// compile time.
/// Tests: `pure_function_no_destinations`
///
/// Unconnected inputs
/// ==================
/// If a function's output is used but one or more of it's inputs is unconnected, then the compiler
/// should throw an error. If for some reason an alternative compiler did not reject this and
/// generated a manifest with no other function sending to that input, then at run-time that
/// functions inputs will never be full and the function will never run. This could produce some
/// form of deadlock or incomplete execution, but it should not produce any run-time error.
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
/// * A function won't be run until all of its inputs are ready
/// * An input maybe initialized at start-up once by a "Once" input initializer
/// * An input maybe initialized after each run by a "Constant" input initializer that ensures that
///   the same value always re-fills the input
///
/// State Transitions
/// =================
///
/// From    To State  Event causing transition and additional conditions          Test
/// ----    --------  --------------------------------------------------          ----
/// Init    Ready     No inputs                                                   `to_ready_1_on_init`
///                   All inputs initialized                                      `to_ready_2_on_init`
///                   All inputs initialized and no destinations                  `to_ready_3_on_init`
/// Init    Waiting   At least one input is not full                              `to_waiting_on_init`
///
/// Ready   Running   `NextJob`: called to fetch the `function_id` for execution      `ready_to_running_on_next`
///
/// Waiting Ready     `Output`: last empty input on a function is filled            `waiting_to_ready_on_input`
///
/// Running Ready     `Output`: it's inputs are all full, so it can run again       `running_to_ready_on_done`
/// Running Waiting   `Output`: it has one input or more empty, to it can't run     `running_to_waiting_on_done`
///
/// Iteration and Recursion
/// =======================
/// A function may send values to itself using a loop-back connector, in order to perform something
/// similar to iteration or recursion, in procedural programming.
///
/// Parallel Execution of Jobs
/// ==========================
/// Multiple functions (jobs) may execute in parallel, providing there is no data dependency
/// preventing it. Example dependencies:
///   * a function lacks an input and needs to get it from another function that has not completed
///
/// Respecting this rule, a `RunTime` can dispatch as many Jobs in parallel as it desires. This one
/// takes the parameter `max_jobs` on `RunState::new()` to specify the maximum number of jobs that
/// are launched in parallel. The minimum value for this is 1
#[derive(Deserialize, Serialize, Clone)]
pub struct RunState {
    /// The `Submission` that lead to this `RunState` object being created
    pub(crate) submission: Submission,
    /// `ready_jobs`: A queue of [Jobs][crate::job::Job] ready to run
    ready_jobs: VecDeque<Job>,
    /// `running_jobs`: set of [Jobs][crate::job::Job] that are running
    running_jobs: HashMap<usize, Job>,
    /// `completed`: [`RuntimeFunction`][flowcore::model::runtime_function::RuntimeFunction]
    /// that have run to completion and won't run again
    completed: HashSet<usize>,
    /// number of jobs sent for execution to date
    number_of_jobs_created: usize,
    /// Track how many busy entries exist per `process_id` (functions and ancestor flows)
    busy_count: HashMap<usize, usize>,
    /// Per-function job creation count (index = `process_id`)
    #[cfg(feature = "metrics")]
    jobs_per_function: Vec<usize>,
    /// Index: parent flow ID → function IDs in that flow (avoids full-manifest scans)
    functions_by_flow: HashMap<usize, Vec<usize>>,
    #[cfg(feature = "trace")]
    #[serde(skip)]
    trace: flowcore::model::trace::Trace,
}

impl RunState {
    /// Create a new `RunState` struct from the list of functions provided and the `Submission`
    /// that was sent to be executed
    #[must_use]
    pub fn new(submission: Submission) -> Self {
        let mut functions_by_flow = HashMap::<usize, Vec<usize>>::new();
        for (id, function) in submission.manifest.functions() {
            functions_by_flow
                .entry(function.get_parent_id())
                .or_default()
                .push(*id);
        }

        #[cfg(feature = "metrics")]
        let num_processes =
            submission.manifest.functions().len() + submission.manifest.flows().len();
        #[cfg(feature = "trace")]
        let trace = crate::trace::topology_from_submission(&submission);
        RunState {
            submission,
            ready_jobs: VecDeque::<Job>::new(),
            running_jobs: HashMap::<usize, Job>::new(),
            completed: HashSet::<usize>::new(),
            number_of_jobs_created: 0,
            busy_count: HashMap::<usize, usize>::new(),
            #[cfg(feature = "metrics")]
            jobs_per_function: vec![0; num_processes],
            functions_by_flow,
            #[cfg(feature = "trace")]
            trace,
        }
    }

    /// Get a reference to the submission
    #[must_use]
    pub fn get_submission(&self) -> &Submission {
        &self.submission
    }

    #[cfg(any(debug_assertions, feature = "debugger"))]
    /// Get a reference to the map of all functions
    #[must_use]
    pub fn get_functions(&self) -> &HashMap<usize, RuntimeFunction> {
        self.submission.manifest.functions()
    }

    #[cfg(feature = "debugger")]
    pub(crate) fn reset(&mut self) {
        debug!("Resetting RunState");
        for function in self.submission.manifest.get_functions().values_mut() {
            function.reset();
        }
        self.ready_jobs.clear();
        self.running_jobs.clear();
        self.completed.clear();
        self.number_of_jobs_created = 0;
        self.busy_count.clear();
        #[cfg(feature = "trace")]
        self.trace.events.clear();
    }

    /// The `ìnit()` function is responsible for initializing all functions, and it returns a
    /// boolean to indicate that it's inputs are fulfilled - and this information is added to the
    /// `RunList` to control the readiness of the Function to be executed.
    ///
    /// After `init` Functions will either be:
    ///    - Ready:   an entry will be added to the `ready` list with this function's id
    ///    - Waiting: function has at least one empty input, so it cannot run. It will not be added to
    ///      the `ready` list, so by omission it is in the `Waiting` state.
    pub(crate) fn init(&mut self) -> Result<()> {
        #[cfg(feature = "debugger")]
        self.reset();

        let mut make_ready_list = vec![];

        debug!("Initializing all functions");
        for function in self.submission.manifest.get_functions().values_mut() {
            function.init();
            if function.can_run() {
                make_ready_list.push((function.id(), function.get_parent_id()));
            }
        }

        for (process_id, parent_id) in make_ready_list {
            self.create_jobs(process_id, parent_id)?;
        }

        #[cfg(feature = "trace")]
        self.record_trace("Init");

        Ok(())
    }

    /// Return the states a function is in
    #[cfg(any(debug_assertions, feature = "debugger", test))]
    #[must_use]
    pub fn get_function_states(&self, function_id: usize) -> Vec<State> {
        let mut states = vec![];

        if self.completed.contains(&function_id) {
            states.push(State::Completed);
        }

        for job in self.running_jobs.values() {
            if job.process_id == function_id {
                states.push(State::Running);
                break;
            }
        }

        for ready_job in &self.ready_jobs {
            if ready_job.process_id == function_id {
                states.push(State::Ready);
            }
        }

        if states.is_empty() {
            states.push(State::Waiting);
        }

        states
    }

    // See if the function is in only the specified state
    #[cfg(test)]
    pub(crate) fn function_state_is_only(&self, function_id: usize, state: &State) -> bool {
        let function_states = self.get_function_states(function_id);
        function_states.len() == 1 && function_states.contains(state)
    }

    /// Get a Set (`job_id`) of the currently running jobs
    #[cfg(any(feature = "debugger", feature = "trace", debug_assertions))]
    #[must_use]
    pub fn get_running(&self) -> &HashMap<usize, Job> {
        &self.running_jobs
    }

    /// Get a reference to the function with `id`
    #[cfg(any(feature = "debugger", test))]
    #[must_use]
    pub fn get_function(&self, id: usize) -> Option<&RuntimeFunction> {
        self.submission.manifest.functions().get(&id)
    }

    pub(crate) fn get_mut(&mut self, id: usize) -> Option<&mut RuntimeFunction> {
        self.submission.manifest.get_functions().get_mut(&id)
    }

    #[cfg(any(debug_assertions, feature = "debugger", feature = "trace"))]
    /// Return the busy count map (`process_id` -> count of busy entries)
    #[must_use]
    pub fn get_busy_count(&self) -> &HashMap<usize, usize> {
        &self.busy_count
    }

    /// Return the set of completed function IDs
    #[cfg(any(feature = "debugger", feature = "trace"))]
    #[must_use]
    pub fn get_completed(&self) -> &HashSet<usize> {
        &self.completed
    }

    // Return a new job to run if there is one and there are not too many jobs already running
    pub(crate) fn get_next_job(&mut self) -> Option<Job> {
        if let Some(limit) = self.submission.max_parallel_jobs {
            if self.number_jobs_running() >= limit {
                trace!("max_parallel_jobs limit of {limit} reached");
                return None;
            }
        }

        self.ready_jobs.remove(0)
    }

    // Update the run_state to reflect that the job is now running
    pub(crate) fn start_job(&mut self, job: Job) {
        self.running_jobs.insert(job.payload.job_id, job);
        #[cfg(feature = "trace")]
        self.record_trace("Dispatch");
    }

    /// Check for running jobs that have exceeded their TTL.
    /// Expired jobs are removed from `running_jobs` and re-queued with an incremented attempt count.
    /// Returns `Err` if any job has exceeded `max_retries`.
    pub(crate) fn requeue_expired_jobs(
        &mut self,
        max_retries: usize,
        #[cfg(feature = "metrics")] metrics: &mut Metrics,
    ) -> Result<()> {
        let now = Instant::now();
        let expired_ids: Vec<usize> = self
            .running_jobs
            .iter()
            .filter(|(_, job)| job.ttl.is_some_and(|ttl| ttl < now))
            .map(|(id, _)| *id)
            .collect();

        for job_id in expired_ids {
            if let Some(mut job) = self.running_jobs.remove(&job_id) {
                if job.attempt > max_retries {
                    return Err(format!(
                        "Job #{} (function #{}) failed after {} retries",
                        job_id,
                        job.process_id,
                        job.attempt - 1
                    )
                    .into());
                }
                warn!(
                    "Job #{} (function #{}) expired after attempt {}, re-queuing",
                    job_id, job.process_id, job.attempt
                );
                job.attempt += 1;
                job.ttl = None;
                #[cfg(feature = "metrics")]
                metrics.increment_jobs_retried();
                self.ready_jobs.push_back(job);
            }
        }

        Ok(())
    }

    /// get the number of jobs created to date in the flow's execution
    #[cfg(any(feature = "metrics", feature = "debugger", feature = "trace"))]
    #[must_use]
    pub fn get_number_of_jobs_created(&self) -> usize {
        self.number_of_jobs_created
    }

    // Complete a Job by taking its output and updating the run-list accordingly.
    //
    // If other functions were blocked trying to send to this one - we can now unblock them
    // as it has consumed its inputs, and they are free to be sent to again.
    //
    // Then, take the output and send it to all destination IOs on different functions.
    // If those other functions have all their inputs, then create jobs for them.
    #[allow(unused_variables, unused_assignments, unused_mut)]
    pub(crate) fn retire_a_job(
        &mut self,
        #[cfg(feature = "metrics")] metrics: &mut Metrics,
        result: (usize, Result<(Option<Value>, RunAgain)>),
        #[cfg(feature = "debugger")] debugger: &mut Debugger,
    ) -> Result<(bool, bool, Job)> {
        let mut display_next_output = false;
        let mut restart = false;

        let mut job = self
            .running_jobs
            .remove(&result.0)
            .ok_or_else(|| format!("Could not find Job#{} to retire it", result.0))?;

        match &result.1 {
            Ok((output_value, function_can_run_again)) => {
                #[cfg(feature = "debugger")]
                debug!(
                    "Job #{}: Function #{} '{}' {:?} -> {:?}",
                    job.payload.job_id,
                    job.process_id,
                    job.function_name,
                    job.payload.input_set,
                    output_value
                );
                #[cfg(not(feature = "debugger"))]
                debug!(
                    "Job #{}: Function #{} {:?} -> {:?}",
                    job.payload.job_id, job.process_id, job.payload.input_set, output_value
                );

                for connection in &job.connections {
                    let value_to_send = match &connection.source {
                        Output(route) => match output_value {
                            Some(output_v) => output_v.pointer(route),
                            None => None,
                        },
                        Input(index) => job.payload.input_set.get(*index),
                    };

                    if let Some(value) = value_to_send {
                        (display_next_output, restart) = self.send_a_value(
                            job.process_id,
                            job.parent_id,
                            connection,
                            value.clone(),
                            #[cfg(feature = "metrics")]
                            metrics,
                            #[cfg(feature = "debugger")]
                            debugger,
                        )?;
                    } else {
                        trace!(
                            "Job #{}:\t\tNo value found at '{}'",
                            job.payload.job_id,
                            connection.source
                        );
                    }
                }

                if *function_can_run_again {
                    let function = self.get_mut(job.process_id).ok_or("No such function")?;

                    // Refill any inputs with function initializers
                    function.init_inputs(InitReason::AfterExecution);

                    // NOTE: The function we are retiring may have new input sets due to sending
                    // to itself via a loopback
                    if function.can_run() {
                        self.create_jobs(job.process_id, job.parent_id)?;
                    }
                    #[cfg(feature = "trace")]
                    self.record_trace("RetireAndSend");
                } else {
                    // otherwise mark it as completed as it will never run again
                    self.mark_as_completed(job.process_id);
                    #[cfg(feature = "trace")]
                    self.record_trace("CompleteJob");
                }
            }
            Err(e) => {
                error!("Error in Job #{}: {e}", job.payload.job_id);
                #[cfg(feature = "trace")]
                self.record_trace("JobError");
            }
        }

        // unblock any senders from other flows that can now run due to this function completing
        // causing the flow to be idle now
        (display_next_output, restart) = self.unblock_flows(
            &job,
            #[cfg(feature = "debugger")]
            debugger,
        )?;

        #[cfg(debug_assertions)]
        checks::check_invariants(self, job.payload.job_id)?;

        trace!(
            "Job #{}: Completed-----------------------",
            job.payload.job_id
        );
        job.result = result.1;

        Ok((display_next_output, restart, job))
    }

    // Send a value produced as part of an output of running a job to a destination function on
    // a specific input, update the metrics and potentially enter the debugger
    fn send_a_value(
        &mut self,
        source_id: usize,
        _source_parent_id: usize,
        connection: &OutputConnection,
        output_value: Value,
        #[cfg(feature = "metrics")] metrics: &mut Metrics,
        #[cfg(feature = "debugger")] debugger: &mut Debugger,
    ) -> Result<(bool, bool)> {
        let mut display_next_output = false;
        let mut restart = false;

        let route_str = match &connection.source {
            Output(route) if route.is_empty() => String::new(),
            Output(route) => format!(" from output route '{route}'"),
            Input(index) => format!(" from Job input #{index}"),
        };

        let loopback = source_id == connection.destination_id;

        if loopback {
            info!("\t\tFunction #{source_id} loopback of value '{output_value}'{route_str} to Self:{}",
                    connection.destination_io_number);
        } else {
            info!(
                "\t\tFunction #{source_id} sending '{output_value}'{route_str} to Function #{}:{}",
                connection.destination_id, connection.destination_io_number
            );
        }

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

        let function = self
            .get_mut(connection.destination_id)
            .ok_or("Could not get function")?;
        let job_count_before = function.input_sets_available();
        if connection.internal {
            function.send_internal(connection.destination_io_number, output_value)?;
        } else {
            function.send(connection.destination_io_number, output_value)?;
        }

        #[cfg(feature = "metrics")]
        metrics.increment_outputs_sent(); // not distinguishing array serialization / wrapping etc.

        let new_job_available = function.input_sets_available() > job_count_before;

        // Loopback sends (function sending to itself) defer job creation
        // until after all other sends complete — handled by the caller.
        if new_job_available && !loopback {
            self.try_create_destination_jobs(connection)?;
        }

        Ok((display_next_output, restart))
    }

    /// Create jobs for a destination function after receiving a value,
    /// unless the send crosses a flow boundary and the destination's
    /// parent flow is already busy (external send gating).
    fn try_create_destination_jobs(&mut self, connection: &OutputConnection) -> Result<()> {
        let dest_flow_busy = !connection.internal
            && self
                .busy_count
                .contains_key(&connection.destination_parent_id);
        if !dest_flow_busy {
            self.create_jobs(connection.destination_id, connection.destination_parent_id)?;
        }
        Ok(())
    }

    /// Return how many jobs are currently running
    #[must_use]
    pub fn number_jobs_running(&self) -> usize {
        self.running_jobs.len()
    }

    /// Return how many jobs are ready to be run, but not running yet
    #[must_use]
    pub fn number_jobs_ready(&self) -> usize {
        self.ready_jobs.len()
    }

    /// Return the ready jobs queue
    #[cfg(any(feature = "debugger", feature = "trace"))]
    #[must_use]
    pub fn get_ready_jobs(&self) -> &VecDeque<Job> {
        &self.ready_jobs
    }

    /// An input blocker is another function that is the only function connected to an empty input
    /// of the target function, and which is not ready to run, hence the target function cannot run.
    ///
    /// # Errors
    /// Returns an error if the target function does not exist.
    #[cfg(feature = "debugger")]
    pub fn get_input_blockers(&self, target_id: usize) -> Result<Vec<usize>> {
        let mut input_blockers = vec![];
        let target_function = self.get_function(target_id).ok_or("No such function")?;

        // for each empty input of the target function
        for (target_io, input) in target_function.inputs().iter().enumerate() {
            if input.values_available() == 0 {
                let mut senders = Vec::<usize>::new();

                // go through all functions to see if sends to the target function on this input
                for sender_function in self.submission.manifest.functions().values() {
                    // if the sender function is not ready to run
                    let mut sender_is_ready = false;

                    for ready_job in &self.ready_jobs {
                        if ready_job.process_id == sender_function.id() {
                            sender_is_ready = true;
                        }
                    }

                    if !sender_is_ready {
                        // for each output route of the sending function, see if the target is
                        // the target function and input
                        for destination in sender_function.get_output_connections() {
                            if (destination.destination_id == target_id)
                                && (destination.destination_io_number == target_io)
                            {
                                senders.push(sender_function.id());
                            }
                        }
                    }
                }

                // If unique sender to this Input, then the target function is waiting for that value
                if senders.len() == 1 {
                    input_blockers.extend(senders);
                }
            }
        }

        Ok(input_blockers)
    }

    // Create one or more new jobs for the function and mark it and ancestor flows as busy
    pub(crate) fn create_jobs(&mut self, process_id: usize, parent_id: usize) -> Result<()> {
        loop {
            self.number_of_jobs_created = self
                .number_of_jobs_created
                .checked_add(1)
                .ok_or("Ran out of job IDs")?;
            let job_id = self.number_of_jobs_created;
            let function = self.get_mut(process_id).ok_or("Could not get function")?;
            if let Some(input_set) = function.take_input_set() {
                let implementation_url = function.get_implementation_url().clone();
                debug!(
                    "Job #{job_id} created for Function #{process_id}({parent_id}) with inputs: {input_set:?}"
                );
                let job = Job::new(
                    process_id,
                    parent_id,
                    #[cfg(feature = "debugger")]
                    function.name().to_string(),
                    Payload {
                        job_id,
                        input_set,
                        implementation_url,
                    },
                    function.get_output_connections().clone(),
                );

                // avoid getting stuck in a loop generating jobs for a function - generate just one
                let always_ready = function.is_always_ready();
                self.ready_jobs.push_back(job);
                #[cfg(feature = "metrics")]
                if let Some(count) = self.jobs_per_function.get_mut(process_id) {
                    *count += 1;
                } else {
                    debug!("process_id {process_id} out of range for jobs_per_function");
                }
                *self.busy_count.entry(process_id).or_insert(0) += 1;
                for ancestor in self.ancestors(parent_id) {
                    *self.busy_count.entry(ancestor).or_insert(0) += 1;
                }
                #[cfg(feature = "trace")]
                self.record_trace("CreateJob");
                if always_ready {
                    return Ok(());
                }
            } else {
                self.number_of_jobs_created = self
                    .number_of_jobs_created
                    .checked_sub(1)
                    .ok_or("Couldn't fix count")?;
                return Ok(());
            }
        }
    }

    /// Return how many functions exist in this flow being executed
    #[cfg(any(feature = "debugger", feature = "metrics"))]
    #[must_use]
    pub fn num_functions(&self) -> usize {
        self.submission.manifest.functions().len()
    }

    /// Total number of processes (functions + flows) in the manifest
    #[must_use]
    pub fn num_processes(&self) -> usize {
        self.submission.manifest.functions().len() + self.submission.manifest.flows().len()
    }

    /// Per-function job creation counts (index = `process_id`)
    #[cfg(feature = "metrics")]
    #[must_use]
    pub fn jobs_per_function(&self) -> &[usize] {
        &self.jobs_per_function
    }

    /// Return the ancestor flow ids starting from `parent_id` up to the root
    fn ancestors(&self, parent_id: usize) -> Vec<usize> {
        let mut result = vec![parent_id];
        let mut current = parent_id;
        while let Some(flow_info) = self.submission.manifest.flows().get(&current) {
            if let Some(pid) = flow_info.parent_id {
                result.push(pid);
                current = pid;
            } else {
                break; // reached root
            }
        }
        result
    }

    // Check if ancestor flows have gone idle and run flow initializers if so
    #[allow(unused_variables, unused_assignments, unused_mut)]
    /// After a job completes, decrement busy counts and check if any
    /// ancestor flow has gone idle.  Matches the spec's `DecrBusy` +
    /// `FlowGoesIdle` sequence.
    fn unblock_flows(
        &mut self,
        job: &Job,
        #[cfg(feature = "debugger")] debugger: &mut Debugger,
    ) -> Result<(bool, bool)> {
        self.remove_from_busy(job.process_id, job.parent_id);
        self.handle_idle_flows(
            job,
            #[cfg(feature = "debugger")]
            debugger,
        )
    }

    /// Check each ancestor flow from innermost to root.  For each newly
    /// idle flow, either create jobs for functions runnable on internal
    /// data (matching the spec's `CreateJob` with `CanRunOnInternal`), or
    /// clear internals and re-apply flow initializers (`FlowGoesIdle`).
    #[allow(unused_variables, unused_assignments, unused_mut)]
    fn handle_idle_flows(
        &mut self,
        job: &Job,
        #[cfg(feature = "debugger")] debugger: &mut Debugger,
    ) -> Result<(bool, bool)> {
        let mut display_next_output = false;
        let mut restart = false;

        for ancestor_id in self.ancestors(job.parent_id) {
            if self.busy_count.contains_key(&ancestor_id) {
                continue;
            }

            if self.has_runnable_on_internal(ancestor_id) {
                debug!(
                    "Job #{}:\tFlow #{} idle but has runnable on internal — creating jobs",
                    job.payload.job_id, ancestor_id
                );
                self.create_jobs_on_internal(ancestor_id)?;
            } else {
                debug!(
                    "Job #{}:\tFlow #{} is now idle — clearing internals and re-initializing",
                    job.payload.job_id, ancestor_id
                );

                #[cfg(feature = "debugger")]
                {
                    (display_next_output, restart) =
                        debugger.check_prior_to_flow_unblock(self, ancestor_id)?;
                }

                self.clear_flow_internal_inputs(ancestor_id);
                self.run_flow_initializers(ancestor_id)?;
                #[cfg(feature = "trace")]
                self.record_trace("FlowGoesIdle");
            }
        }

        Ok((display_next_output, restart))
    }

    /// Create jobs for all runnable functions in a flow that can run
    /// on internal data.
    fn create_jobs_on_internal(&mut self, flow_id: usize) -> Result<()> {
        let runnable: Vec<_> = self
            .functions_by_flow
            .get(&flow_id)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter(|id| {
                !self.completed.contains(id)
                    && self
                        .submission
                        .manifest
                        .functions()
                        .get(id)
                        .is_some_and(RuntimeFunction::can_run_on_internal)
            })
            .collect();
        for func_id in runnable {
            self.create_jobs(func_id, flow_id)?;
        }
        Ok(())
    }

    // Decrement busy_count for the function and all its ancestor flows
    fn remove_from_busy(&mut self, process_id: usize, parent_id: usize) {
        // Decrement function's own count
        if let Some(count) = self.busy_count.get_mut(&process_id) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                self.busy_count.remove(&process_id);
            }
        }
        // Decrement ancestor flow counts
        for ancestor in self.ancestors(parent_id) {
            if let Some(count) = self.busy_count.get_mut(&ancestor) {
                *count = count.saturating_sub(1);
                if *count == 0 {
                    self.busy_count.remove(&ancestor);
                }
            }
        }
        trace!("\t\t\tUpdated busy_count to: {:?}", self.busy_count);
    }

    fn has_runnable_on_internal(&self, flow_id: usize) -> bool {
        let result = self.functions_by_flow
            .get(&flow_id)
            .is_some_and(|func_ids| {
                func_ids.iter().any(|id| {
                    let not_completed = !self.completed.contains(id);
                    let can_run = self
                        .submission
                        .manifest
                        .functions()
                        .get(id)
                        .is_some_and(RuntimeFunction::can_run_on_internal);
                    if not_completed && !can_run {
                        if let Some(f) = self.submission.manifest.functions().get(id) {
                            debug!(
                                "\tFlow #{flow_id}: Function #{id} '{}' cannot run on internal: inputs={:?}",
                                f.name(),
                                f.input_counts()
                            );
                        }
                    }
                    not_completed && can_run
                })
            });
        debug!("\tFlow #{flow_id} has_runnable_on_internal = {result}");
        result
    }

    fn clear_flow_internal_inputs(&mut self, flow_id: usize) {
        if let Some(func_ids) = self.functions_by_flow.get(&flow_id).cloned() {
            for id in &func_ids {
                if !self.completed.contains(id) {
                    if let Some(f) = self.submission.manifest.get_functions().get_mut(id) {
                        let counts = f.input_counts();
                        let has_values = counts.iter().any(|(total, _)| *total > 0);
                        if has_values {
                            debug!(
                                "\tClearing internal inputs of Function #{id} '{}' in Flow #{flow_id}: {:?}",
                                f.name(),
                                counts
                            );
                        }
                        f.clear_internal_inputs();
                    }
                }
            }
        }
    }

    fn run_flow_initializers(&mut self, flow_id: usize) -> Result<()> {
        let mut runnable_functions = Vec::<usize>::new();
        if let Some(func_ids) = self.functions_by_flow.get(&flow_id).cloned() {
            for id in &func_ids {
                if !self.completed.contains(id) {
                    if let Some(f) = self.submission.manifest.get_functions().get_mut(id) {
                        f.init_inputs(InitReason::FlowIdle);
                        if f.can_run() {
                            runnable_functions.push(*id);
                        }
                    }
                }
            }
        }

        for function_id in runnable_functions {
            self.create_jobs(function_id, flow_id)?;
        }

        Ok(())
    }

    // Mark a function (via its ID) as having run to completion
    pub(crate) fn mark_as_completed(&mut self, function_id: usize) {
        self.completed.insert(function_id);
    }

    #[cfg(feature = "trace")]
    fn record_trace(&mut self, action: &str) {
        crate::trace::record_event(
            &mut self.trace,
            action,
            &self.submission.manifest,
            &self.busy_count,
            &self.ready_jobs,
            &self.running_jobs,
            &self.completed,
            self.number_of_jobs_created,
        );
    }

    /// Extract the accumulated trace, replacing it with an empty trace
    #[cfg(feature = "trace")]
    #[allow(dead_code)]
    pub(crate) fn take_trace(&mut self) -> flowcore::model::trace::Trace {
        std::mem::replace(
            &mut self.trace,
            crate::trace::topology_from_submission(&self.submission),
        )
    }

    /// Write the trace to the path specified by the `FLOW_TRACE` env var
    #[cfg(feature = "trace")]
    pub(crate) fn write_trace(&mut self) -> flowcore::errors::Result<()> {
        if let Ok(trace_path) = std::env::var("FLOW_TRACE") {
            let trace = self.take_trace();
            let trace_json = trace.to_json();
            std::fs::write(&trace_path, &trace_json)
                .map_err(|e| format!("Could not write trace to {trace_path}: {e}"))?;
        }
        Ok(())
    }
}

impl fmt::Display for RunState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "         Submission:\n{}", self.submission)?;

        writeln!(f, "RunState:")?;
        writeln!(f, "          Jobs Created: {}", self.number_of_jobs_created)?;
        writeln!(f, "Number of Jobs Running: {}", self.running_jobs.len())?;
        writeln!(f, "          Jobs Running: {:?}", self.running_jobs.keys())?;
        writeln!(
            f,
            "       Functions Ready: {:?}",
            self.ready_jobs
                .iter() // jonesy:allow(capacity)
                .map(|j| j.payload.job_id)
                .collect::<Vec<usize>>()
        )?;
        writeln!(f, "   Functions Completed: {:?}", self.completed)?;
        write!(f, "            Busy Functions: {:?}", self.busy_count)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {
    use serde_json::{json, Value};
    use url::Url;

    use flowcore::errors::Result;
    use flowcore::model::flow_manifest::FlowManifest;
    use flowcore::model::input::Input;
    use flowcore::model::input::InputInitializer::Once;
    use flowcore::model::metadata::MetaData;
    use flowcore::model::output_connection::{OutputConnection, Source};
    use flowcore::model::runtime_function::RuntimeFunction;
    use flowcore::model::submission::Submission;

    #[cfg(feature = "debugger")]
    use crate::debug_command::DebugCommand;
    #[cfg(feature = "debugger")]
    use crate::debugger::Debugger;
    #[cfg(feature = "debugger")]
    use crate::debugger_handler::DebuggerHandler;

    use super::RunState;
    use super::State;
    use super::{Job, Payload};

    fn test_function_a_to_b_not_init() -> RuntimeFunction {
        let connection_to_f1 = OutputConnection::new(
            Source::default(),
            1,
            0,
            0,
            true,
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
                #[cfg(feature = "debugger")]
                "",
                0,
                false,
                None,
                None,
            )],
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
            true,
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
                #[cfg(feature = "debugger")]
                "",
                0,
                false,
                Some(Once(json!(1))),
                None,
            )],
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
                #[cfg(feature = "debugger")]
                "",
                0,
                false,
                Some(Once(json!(1))),
                None,
            )],
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
                #[cfg(feature = "debugger")]
                "",
                0,
                false,
                None,
                None,
            )],
            1,
            0,
            &[],
            false,
        )
    }

    fn test_job(source_process_id: usize, destination_process_id: usize) -> Job {
        let out_conn = OutputConnection::new(
            Source::default(),
            destination_process_id,
            0,
            0,
            true,
            String::default(),
            #[cfg(feature = "debugger")]
            String::default(),
        );
        Job {
            process_id: source_process_id,
            parent_id: 0,
            #[cfg(feature = "debugger")]
            function_name: String::new(),
            connections: vec![out_conn],
            payload: Payload {
                job_id: 1,
                implementation_url: Url::parse("file://test").expect("Could not parse Url"),
                input_set: vec![json!(1)],
            },
            result: Ok((Some(json!(1)), true)),
            ttl: None,
            attempt: 1,
        }
    }

    #[cfg(feature = "debugger")]
    struct DummyServer;

    #[cfg(feature = "debugger")]
    impl DebuggerHandler for DummyServer {
        fn start(&mut self) {}
        fn job_breakpoint(&mut self, _job: &Job, _function: &RuntimeFunction, _states: Vec<State>) {
        }
        fn flow_unblock_breakpoint(&mut self, _flow_id: usize) {}
        fn send_breakpoint(
            &mut self,
            _: &str,
            _source_process_id: usize,
            _output_route: &str,
            _value: &Value,
            _destination_id: usize,
            _destination_name: &str,
            _input_name: &str,
            _input_number: usize,
        ) {
        }
        fn job_error(&mut self, _job: &Job) {}
        fn job_completed(&mut self, _job: &Job) {}
        fn outputs(&mut self, _output: Vec<OutputConnection>) {}
        fn input(&mut self, _input: Input) {}
        fn function_list(&mut self, _functions: &[RuntimeFunction]) {}
        fn function_states(&mut self, _: RuntimeFunction, _: Vec<State>, _: Vec<usize>) {}
        fn inspect_function(&mut self, _: usize, _: &RunState) {}
        fn run_state(&mut self, _run_state: &RunState) {}
        fn message(&mut self, _message: String) {}
        fn breakpoint_list(&mut self, _breakpoints: Vec<crate::debug_command::BreakpointSpec>) {}
        fn panic(&mut self, _state: &RunState, _error_message: String) {}
        fn debugger_exiting(&mut self) {}
        fn debugger_resetting(&mut self) {}
        fn debugger_error(&mut self, _error: String) {}
        fn execution_starting(&mut self) {}
        fn execution_ended(&mut self) {}
        fn process_tree(&mut self, _: &RunState) {}
        fn inspect_by_state(&mut self, _: &str, _: &RunState) {}
        fn inspect_flow(&mut self, _: usize, _: &RunState) {}
        fn job_inspect(&mut self, _: Job) {}
        #[cfg(feature = "metrics")]
        fn execution_metrics(&mut self, _: flowcore::model::metrics::Metrics) {}
        fn flow_list(&mut self, _: &[usize], _: &RunState) {}
        fn get_command(&mut self, _state: &RunState) -> Result<DebugCommand> {
            unimplemented!();
        }
    }

    #[cfg(feature = "debugger")]
    fn dummy_debugger(server: &mut dyn DebuggerHandler) -> Debugger<'_> {
        Debugger::new(server)
    }

    fn test_meta_data() -> MetaData {
        MetaData {
            name: "test".into(),
            version: "0.0.0".into(),
            description: "a test".into(),
            authors: vec!["me".into()],
        }
    }

    fn test_manifest(functions: Vec<RuntimeFunction>) -> FlowManifest {
        let mut manifest = FlowManifest::new(test_meta_data());
        for function in functions {
            manifest.add_function(function);
        }
        manifest
    }

    fn test_submission(functions: Vec<RuntimeFunction>) -> Submission {
        Submission::new(
            test_manifest(functions),
            None,
            None,
            #[cfg(feature = "debugger")]
            true,
        )
    }

    mod general_run_state_tests {
        use super::super::RunState;

        #[test]
        fn display_run_state_test() {
            let f_a = super::test_function_a_to_b();
            let f_b = super::test_function_b_not_init();
            let mut state = RunState::new(super::test_submission(vec![f_a, f_b]));
            state.init().expect("Could not init state");

            #[cfg(any(feature = "debugger", feature = "metrics"))]
            assert_eq!(state.num_functions(), 2);

            println!("Run state: {state}");
        }

        #[cfg(feature = "metrics")]
        #[test]
        fn jobs_created_zero_at_init() {
            let mut state = RunState::new(super::test_submission(vec![]));
            state.init().expect("Could not init state");
            assert_eq!(
                0,
                state.get_number_of_jobs_created(),
                "At init jobs() should be 0"
            );
            assert_eq!(0, state.number_jobs_ready());
        }

        #[cfg(feature = "debugger")]
        #[test]
        fn zero_running_at_init() {
            let mut state = RunState::new(super::test_submission(vec![]));
            state.init().expect("Could not init state");
            assert!(
                state.get_running().is_empty(),
                "At init get_running() should be empty"
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

        use crate::run_state::test::test_function_b_not_init;

        use super::super::RunState;
        use super::super::State;
        use super::super::{Job, Payload};

        #[test]
        fn to_ready_1_on_init() {
            let f_a = super::test_function_a_to_b();
            let f_b = test_function_b_not_init();
            let mut state = RunState::new(super::test_submission(vec![f_a, f_b]));

            // Event
            state.init().expect("Could not init state");

            // Test
            assert!(
                state.function_state_is_only(0, &State::Ready),
                "f_a should be Ready"
            );
            assert_eq!(1, state.number_jobs_ready());
            assert!(
                state.function_state_is_only(1, &State::Waiting),
                "f_b should be waiting for input"
            );
        }

        #[test]
        fn input_blocker() {
            let f_a = super::test_function_a_to_b_not_init();
            let f_b = test_function_b_not_init();
            let mut state = RunState::new(super::test_submission(vec![f_a, f_b]));

            // Event
            state.init().expect("Could not init state");

            // Test
            assert!(
                state.function_state_is_only(0, &State::Waiting),
                "f_a should be waiting for input"
            );
            assert!(
                state.function_state_is_only(1, &State::Waiting),
                "f_b should be waiting for input"
            );
            #[cfg(feature = "debugger")]
            assert!(
                state
                    .get_input_blockers(1)
                    .expect("Could not get blockers")
                    .contains(&0),
                "There should be an input blocker"
            );
        }

        #[test]
        fn to_ready_2_on_init() {
            let f_a = super::test_function_a_to_b();
            let f_b = test_function_b_not_init();
            let mut state = RunState::new(super::test_submission(vec![f_a, f_b]));

            // Event
            state.init().expect("Could not init state");

            // Test
            assert!(
                state.function_state_is_only(0, &State::Ready),
                "f_a should be Ready"
            );
        }

        #[test]
        fn to_ready_3_on_init() {
            let f_a = super::test_function_a_init();
            let mut state = RunState::new(super::test_submission(vec![f_a]));

            // Event
            state.init().expect("Could not init state");

            // Test
            assert!(
                state.function_state_is_only(0, &State::Ready),
                "f_a should be Ready"
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
                    #[cfg(feature = "debugger")]
                    "",
                    0,
                    false,
                    None,
                    None,
                )],
                0,
                0,
                &[],
                false,
            )
        }

        #[test]
        fn to_waiting_on_init() {
            let f_a = test_function_a_not_init();
            let mut state = RunState::new(super::test_submission(vec![f_a]));

            // Event
            state.init().expect("Could not init state");

            // Test
            assert!(
                state.function_state_is_only(0, &State::Waiting),
                "f_a should be Waiting"
            );
        }

        #[test]
        fn ready_to_running_on_next() {
            let f_a = super::test_function_a_init();
            let mut state = RunState::new(super::test_submission(vec![f_a]));
            state.init().expect("Could not init state");
            assert!(
                state.function_state_is_only(0, &State::Ready),
                "f_a should be Ready"
            );

            // Event
            let job = state.get_next_job().expect("Couldn't get next job");
            state.start_job(job.clone());

            // Test
            state
                .running_jobs
                .get(&job.payload.job_id)
                .expect("Job should have been running");
        }

        #[test]
        fn unready_not_to_running_on_next() {
            let f_a = test_function_a_not_init();
            let mut state = RunState::new(super::test_submission(vec![f_a]));
            state.init().expect("Could not init state");
            assert!(
                state.function_state_is_only(0, &State::Waiting),
                "f_a should be Waiting"
            );

            // Event
            assert!(
                state.get_next_job().is_none(),
                "next_job() should return None"
            );

            // Test
            assert!(
                state.function_state_is_only(0, &State::Waiting),
                "f_a should be Waiting"
            );
        }

        fn test_job() -> Job {
            Job {
                process_id: 0,
                #[cfg(feature = "debugger")]
                function_name: String::new(),
                parent_id: 0,
                connections: vec![],
                payload: Payload {
                    job_id: 1,
                    implementation_url: Url::parse("file://test").expect("Could not parse Url"),
                    input_set: vec![json!(1)],
                },
                result: Ok((None, true)),
                ttl: None,
                attempt: 1,
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
                    #[cfg(feature = "debugger")]
                    "",
                    0,
                    false,
                    Some(Always(json!(1))),
                    None,
                )],
                0,
                0,
                &[],
                false,
            );

            let mut state = RunState::new(super::test_submission(vec![f_a]));
            #[cfg(feature = "metrics")]
            let mut metrics = Metrics::new(1, 1);
            #[cfg(feature = "debugger")]
            let mut server = super::DummyServer {};
            #[cfg(feature = "debugger")]
            let mut debugger = super::dummy_debugger(&mut server);

            state.init().expect("Could not init state");
            assert!(
                state.function_state_is_only(0, &State::Ready),
                "f_a should be Ready"
            );
            let job = state.get_next_job().expect("Couldn't get next job");
            assert_eq!(
                0, job.process_id,
                "get_next_job() should return process_id = 0"
            );
            state.start_job(job.clone());

            state
                .running_jobs
                .get(&job.payload.job_id)
                .expect("Job with f_a should be Running");

            // Event
            let job = test_job();
            state
                .retire_a_job(
                    #[cfg(feature = "metrics")]
                    &mut metrics,
                    (job.payload.job_id, job.result),
                    #[cfg(feature = "debugger")]
                    &mut debugger,
                )
                .expect("Problem retiring job");

            // Test
            assert!(
                state.function_state_is_only(0, &State::Ready),
                "f_a should be Ready again"
            );
        }

        // Done: it has one input or more empty, to it can't run
        #[test]
        #[serial]
        fn running_to_waiting_on_done() {
            let f_a = super::test_function_a_init();

            let mut state = RunState::new(super::test_submission(vec![f_a]));
            #[cfg(feature = "metrics")]
            let mut metrics = Metrics::new(1, 1);
            #[cfg(feature = "debugger")]
            let mut server = super::DummyServer {};
            #[cfg(feature = "debugger")]
            let mut debugger = super::dummy_debugger(&mut server);

            state.init().expect("Could not init state");
            assert!(
                state.function_state_is_only(0, &State::Ready),
                "f_a should be Ready"
            );
            let job = state.get_next_job().expect("Couldn't get next job");
            assert_eq!(0, job.process_id, "next() should return process_id = 0");
            state.start_job(job.clone());

            state
                .running_jobs
                .get(&job.payload.job_id)
                .expect("Job with f_a should be Running");

            // Event
            let job = test_job();
            state
                .retire_a_job(
                    #[cfg(feature = "metrics")]
                    &mut metrics,
                    (job.payload.job_id, job.result),
                    #[cfg(feature = "debugger")]
                    &mut debugger,
                )
                .expect("Problem retiring job");

            // Test
            assert!(
                state.function_state_is_only(0, &State::Waiting),
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
                true,
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
                    #[cfg(feature = "debugger")]
                    "",
                    0,
                    false,
                    None,
                    None,
                )],
                1,
                0,
                &[out_conn],
                false,
            );
            let mut state = RunState::new(super::test_submission(vec![f_a, f_b]));
            #[cfg(feature = "metrics")]
            let mut metrics = Metrics::new(1, 1);
            #[cfg(feature = "debugger")]
            let mut server = super::DummyServer {};
            #[cfg(feature = "debugger")]
            let mut debugger = super::dummy_debugger(&mut server);

            state.init().expect("Could not init state");
            assert!(
                state.function_state_is_only(0, &State::Waiting),
                "f_a should be Waiting"
            );

            // Event run f_b which will send to f_a
            let job = super::test_job(1, 0);
            state.start_job(job.clone());

            state
                .retire_a_job(
                    #[cfg(feature = "metrics")]
                    &mut metrics,
                    (job.payload.job_id, job.result),
                    #[cfg(feature = "debugger")]
                    &mut debugger,
                )
                .expect("Problem retiring job");

            // Test
            assert!(
                state.function_state_is_only(0, &State::Ready),
                "f_a should be Ready"
            );
        }

        /*
            fA (#0) has an input but not initialized, outputs to #1 (fB)
            fB (#1) has an input with a ConstantInitializer, outputs back to #0 (fA)
        */
        #[test]
        #[serial]
        fn waiting_to_ready_via_output() {
            let f_a = super::test_function_a_to_b_not_init();
            let connection_to_f0 = OutputConnection::new(
                Source::default(),
                0,
                0,
                0,
                true,
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
                    #[cfg(feature = "debugger")]
                    "",
                    0,
                    false,
                    Some(Always(json!(1))),
                    None,
                )],
                1,
                0,
                &[connection_to_f0],
                false,
            );
            let mut state = RunState::new(super::test_submission(vec![f_a, f_b]));
            #[cfg(feature = "metrics")]
            let mut metrics = Metrics::new(1, 1);
            #[cfg(feature = "debugger")]
            let mut server = super::DummyServer {};
            #[cfg(feature = "debugger")]
            let mut debugger = super::dummy_debugger(&mut server);

            state.init().expect("Could not init state");

            assert!(
                state.function_state_is_only(1, &State::Ready),
                "f_b should be Ready"
            );
            assert!(
                state.function_state_is_only(0, &State::Waiting),
                "f_a should be in Waiting"
            );

            // create output from f_b as if it had run - will send to f_a
            let job = super::test_job(1, 0);
            state.start_job(job.clone());

            state
                .retire_a_job(
                    #[cfg(feature = "metrics")]
                    &mut metrics,
                    (job.payload.job_id, job.result),
                    #[cfg(feature = "debugger")]
                    &mut debugger,
                )
                .expect("Problem retiring job");

            // Test
            assert!(
                state.function_state_is_only(0, &State::Ready),
                "f_a should be Ready"
            );
        }
    }

    /****************************** Miscellaneous tests **************************/
    mod functional_tests {
        // Tests using Debugger (and hence Client/Server connection) need to be executed in parallel
        // to avoid multiple trying to bind to the same socket at the same time
        use serial_test::serial;

        use flowcore::model::input::Input;
        #[cfg(feature = "metrics")]
        use flowcore::model::metrics::Metrics;
        use flowcore::model::output_connection::{OutputConnection, Source};
        use flowcore::model::runtime_function::RuntimeFunction;

        use super::super::RunState;

        fn test_functions() -> Vec<RuntimeFunction> {
            let out_conn1 = OutputConnection::new(
                Source::default(),
                1,
                0,
                0,
                true,
                String::default(),
                #[cfg(feature = "debugger")]
                String::default(),
            );
            let out_conn2 = OutputConnection::new(
                Source::default(),
                2,
                0,
                0,
                true,
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
                    #[cfg(feature = "debugger")]
                    "",
                    0,
                    false,
                    None,
                    None,
                )], // inputs array
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
                    #[cfg(feature = "debugger")]
                    "",
                    0,
                    false,
                    None,
                    None,
                )], // inputs array
                2,
                0,
                &[],
                false,
            );
            vec![p0, p1, p2]
        }

        #[test]
        fn get_works() {
            let state = RunState::new(super::test_submission(test_functions()));
            let got = state
                .get_function(1)
                .ok_or("Could not get function by id")
                .expect("Could not get function with that id");
            assert_eq!(got.id(), 1);
        }

        #[test]
        fn no_next_if_none_ready() {
            let mut state = RunState::new(super::test_submission(test_functions()));
            assert!(state.get_next_job().is_none());
        }

        #[test]
        fn next_works() {
            let mut state = RunState::new(super::test_submission(test_functions()));

            // Put 0 on the ready list
            state.create_jobs(0, 0).expect("Could not create jobs");

            state.get_next_job().expect("Couldn't get next job");
        }

        #[test]
        fn inputs_ready_makes_ready() {
            let mut state = RunState::new(super::test_submission(test_functions()));

            // Put 0 on the ready list
            state.create_jobs(0, 0).expect("Could not create jobs");

            state.get_next_job().expect("Couldn't get next job");
        }

        #[test]
        #[serial]
        fn wont_return_too_many_jobs() {
            let mut state = RunState::new(super::test_submission(test_functions()));

            state.init().expect("Could not init state");

            let _ = state.get_next_job().expect("Couldn't get next job");

            assert!(
                state.get_next_job().is_none(),
                "Did not expect a Ready job!"
            );
        }

        /*
            This test checks that a function with no output destinations (even if pure and produces
            some output) can be executed and nothing crashes
        */
        #[test]
        #[serial]
        fn pure_function_no_destinations() {
            let f_a = super::test_function_a_init();
            let _id = f_a.id();

            let mut state = RunState::new(super::test_submission(vec![f_a]));
            #[cfg(feature = "metrics")]
            let mut metrics = Metrics::new(1, 1);
            #[cfg(feature = "debugger")]
            let mut server = super::DummyServer {};
            #[cfg(feature = "debugger")]
            let mut debugger = super::dummy_debugger(&mut server);

            state.init().expect("Could not init state");

            let job = state.get_next_job().expect("Couldn't get next job");
            state.start_job(job.clone());

            // Test there is no problem producing an Output when no destinations to send it to
            state
                .retire_a_job(
                    #[cfg(feature = "metrics")]
                    &mut metrics,
                    (job.payload.job_id, job.result),
                    #[cfg(feature = "debugger")]
                    &mut debugger,
                )
                .expect("Failed to retire job correctly");
        }

        #[test]
        #[serial]
        #[allow(clippy::indexing_slicing)]
        fn external_send_to_busy_subflow_does_not_create_job() {
            use flowcore::model::input::InputInitializer::Always;
            use serde_json::json;
            // Function A (parent=0) outputs to Function B (parent=1) via external connection
            let external_conn = OutputConnection::new(
                Source::default(),
                1,     // destination_id = function B
                0,     // destination_io_number
                1,     // destination_parent_id = flow 1 (different from A's parent 0)
                false, // internal = false (crosses flow boundary)
                "/fB".to_string(),
                #[cfg(feature = "debugger")]
                String::default(),
            );
            let f_a = RuntimeFunction::new(
                #[cfg(feature = "debugger")]
                "fA",
                #[cfg(feature = "debugger")]
                "/fA",
                "file://fake/test",
                vec![Input::new(
                    #[cfg(feature = "debugger")]
                    "",
                    0,
                    false,
                    Some(Always(json!(1))),
                    None,
                )],
                0, // process_id
                0, // parent_id (flow 0)
                &[external_conn],
                false,
            );
            let f_b = RuntimeFunction::new(
                #[cfg(feature = "debugger")]
                "fB",
                #[cfg(feature = "debugger")]
                "/fB",
                "file://fake/test",
                vec![Input::new(
                    #[cfg(feature = "debugger")]
                    "",
                    0,
                    false,
                    None,
                    None,
                )],
                1, // process_id
                1, // parent_id (flow 1 — different sub-flow)
                &[],
                false,
            );

            let mut state = RunState::new(super::test_submission(vec![f_a, f_b]));
            #[cfg(feature = "metrics")]
            let mut metrics = Metrics::new(2, 2);
            #[cfg(feature = "debugger")]
            let mut server = super::DummyServer {};
            #[cfg(feature = "debugger")]
            let mut debugger = super::dummy_debugger(&mut server);

            state.init().expect("Could not init state");

            // A is ready (has Always initializer), B is not
            let job = state.get_next_job().expect("Couldn't get next job");
            assert_eq!(job.process_id, 0, "First job should be for function A");
            state.start_job(job.clone());

            // Now flow 1 (B's parent) is NOT busy, but flow 0 IS busy (A is running)
            // Mark flow 1 as busy to simulate a sub-flow that's already running
            *state.busy_count.entry(1).or_insert(0) += 1;

            // Retire A's job — it sends output to B via external connection
            // Since B's parent flow (1) is busy, no job should be created for B
            state
                .retire_a_job(
                    #[cfg(feature = "metrics")]
                    &mut metrics,
                    (job.payload.job_id, Ok((Some(json!(42)), true))),
                    #[cfg(feature = "debugger")]
                    &mut debugger,
                )
                .expect("Failed to retire job");

            // B should NOT have a job created because its parent flow is busy
            // A may be re-queued (has always initializer), so check B specifically
            let b_has_job = state.ready_jobs.iter().any(|j| j.process_id == 1);
            assert!(
                !b_has_job,
                "No job should be created for B when its parent sub-flow is busy"
            );
        }
    }
}

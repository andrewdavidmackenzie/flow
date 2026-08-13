#[cfg(all(not(feature = "debugger"), not(feature = "submission")))]
use std::marker::PhantomData;
use std::time::{Duration, Instant};

use log::{debug, error, info, trace};
use serde_json::Value;

use flowcore::errors::Result;
#[cfg(feature = "metrics")]
use flowcore::model::metrics::Metrics;
use flowcore::model::submission::Submission;
use flowcore::RunAgain;

#[cfg(feature = "metrics")]
use std::sync::atomic::AtomicU64;

use crate::debug_action::DebugAction;
#[cfg(feature = "debugger")]
use crate::debugger::Debugger;
#[cfg(feature = "debugger")]
use crate::debugger_handler::DebuggerHandler;
use crate::dispatcher::Dispatcher;
use crate::job::Job;
use crate::run_state::RunState;
#[cfg(feature = "submission")]
use crate::submission_handler::SubmissionHandler;

/// The `Coordinator` coordinates the dispatching of jobs for flow execution.
///
/// A Job consists of a set of Input values and an Implementation of a Function for execution,
/// gathering the resulting Outputs and distributing output values to other connected function's
/// Inputs.
///
/// It accepts Flows to be executed in the form of a `Submission` struct that has the required
/// information to execute the flow.
pub struct Coordinator<'a> {
    /// A `Server` to communicate with clients
    #[cfg(feature = "submission")]
    submission_handler: &'a mut dyn SubmissionHandler,
    /// Dispatcher to dispatch jobs for execution
    dispatcher: Dispatcher,
    /// Maximum time to wait for a job result before considering it lost
    job_timeout: Option<Duration>,
    /// Base URL of the WASM HTTP server. When set, `file://` implementation
    /// URLs in job payloads are rewritten to `http://` URLs so remote executors
    /// can fetch WASM modules.
    wasm_base_url: Option<url::Url>,
    /// Local job/results addresses for the executor threads.
    /// Used to reconnect executors back to local after helping a remote coordinator.
    local_job_address: Option<String>,
    local_results_address: Option<String>,
    /// Remote job/results addresses discovered via mDNS.
    /// When set, executors connect to these while idle (no local flow running).
    remote_job_address: Option<String>,
    remote_results_address: Option<String>,
    /// Whether executor threads are currently connected to a remote coordinator
    executors_remote: bool,
    /// Whether the executor has `ContextIO` configured for context proxying
    has_context_proxy: bool,
    /// Shared sub-flow manifest registry from the executor.
    subflow_registry: Option<crate::executor::SubflowRegistry>,
    #[cfg(feature = "debugger")]
    /// A `Debugger` to communicate with debug clients
    debugger: Debugger<'a>,
    #[cfg(all(not(feature = "debugger"), not(feature = "submission")))]
    _data: PhantomData<&'a Dispatcher>,
}

#[cfg(feature = "metrics")]
static TOTAL_GET_RESULT_US: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "metrics")]
static TOTAL_RETIRE_JOB_US: AtomicU64 = AtomicU64::new(0);

/// RAII guard that accumulates elapsed microseconds into an `AtomicU64` on drop.
#[cfg(feature = "metrics")]
struct MetricsTimer {
    start: Instant,
    target: &'static AtomicU64,
}

#[cfg(feature = "metrics")]
impl MetricsTimer {
    fn new(target: &'static AtomicU64) -> Self {
        Self {
            start: Instant::now(),
            target,
        }
    }
}

#[cfg(feature = "metrics")]
impl Drop for MetricsTimer {
    fn drop(&mut self) {
        self.target.fetch_add(
            self.start
                .elapsed()
                .as_micros()
                .try_into()
                .unwrap_or(u64::MAX),
            std::sync::atomic::Ordering::Relaxed,
        );
    }
}

/// Result of running the inner dispatch/retire loop for one iteration of flow execution.
/// Tells the outer loop whether to restart (debugger reset) or finish.
enum FlowIterationResult {
    /// Flow execution completed normally or was stopped by the client
    Done,
    /// The debugger requested a restart of flow execution
    Restart,
}

/// Returns true if `err` is the error raised when the debug client ends the debug session
/// (e.g. the 'e' command). This is handled specially so the client is still notified that the
/// flow has ended and can exit cleanly.
#[cfg(feature = "debugger")]
fn is_debugger_exit_error(err: &flowcore::errors::Error) -> bool {
    err.to_string() == crate::debugger::DEBUGGER_EXIT_MESSAGE
}

/// When the debugger feature is disabled this error can never be raised, so never matches.
#[cfg(not(feature = "debugger"))]
fn is_debugger_exit_error(_err: &flowcore::errors::Error) -> bool {
    false
}

// --- Debugger wrapper methods ---
// Encapsulate `#[cfg(feature = "debugger")]` branching so the main coordinator
// logic reads cleanly without conditional compilation noise.
#[allow(unused_variables)]
impl Coordinator<'_> {
    fn debugger_start(&mut self, state: &RunState) {
        #[cfg(feature = "debugger")]
        if state.submission.debug_enabled {
            self.debugger.start();
        }
    }

    fn debugger_wait_if_enabled(&mut self, state: &mut RunState) -> Result<bool> {
        #[cfg(feature = "debugger")]
        if state.submission.debug_enabled {
            let action = self.debugger.wait_for_command(state)?;
            return Ok(action.should_restart());
        }
        Ok(false)
    }

    fn debugger_check_mid_loop(&mut self, state: &mut RunState) -> Result<bool> {
        #[cfg(all(feature = "debugger", feature = "submission"))]
        if state.submission.debug_enabled && self.submission_handler.should_enter_debugger()? {
            let action = self.debugger.wait_for_command(state)?;
            return Ok(action.should_restart());
        }
        Ok(false)
    }

    fn debugger_check_before_job(
        &mut self,
        state: &mut RunState,
        job: &Job,
    ) -> Result<DebugAction> {
        #[cfg(feature = "debugger")]
        return self.debugger.check_prior_to_job(state, job);
        #[cfg(not(feature = "debugger"))]
        Ok(DebugAction::Continue)
    }

    fn debugger_job_done(&mut self, action: &mut DebugAction, state: &mut RunState, job: &Job) {
        #[cfg(feature = "debugger")]
        if action.should_display() {
            *action = self.debugger.job_done(state, job);
        }
    }

    fn debugger_error(
        &mut self,
        state: &mut RunState,
        err: &flowcore::errors::Error,
    ) -> Result<DebugAction> {
        #[cfg(feature = "debugger")]
        if state.submission.debug_enabled {
            return self.debugger.error(state, err.to_string());
        }
        Ok(DebugAction::Continue)
    }
}

impl<'a> Coordinator<'a> {
    const MAX_JOB_RETRIES: usize = 3;

    /// Create a new `coordinator` with `num_threads` local executor threads
    pub fn new(
        dispatcher: Dispatcher,
        #[cfg(feature = "submission")] submitter: &'a mut dyn SubmissionHandler,
        #[cfg(feature = "debugger")] debug_server: &'a mut dyn DebuggerHandler,
    ) -> Self {
        Coordinator {
            #[cfg(feature = "submission")]
            submission_handler: submitter,
            dispatcher,
            job_timeout: None,
            wasm_base_url: None,
            local_job_address: None,
            local_results_address: None,
            remote_job_address: None,
            remote_results_address: None,
            executors_remote: false,
            has_context_proxy: false,
            subflow_registry: None,
            #[cfg(feature = "debugger")]
            debugger: Debugger::new(debug_server),
            #[cfg(all(not(feature = "debugger"), not(feature = "submission")))]
            _data: PhantomData,
        }
    }

    /// Set the WASM HTTP server base URL. When set, `file://` implementation
    /// URLs in dispatched job payloads are rewritten to `http://` URLs so
    /// remote executors can fetch WASM modules.
    pub fn set_wasm_base_url(&mut self, base_url: url::Url) {
        self.wasm_base_url = Some(base_url);
    }

    /// Configure local executor addresses (for reconnecting back after remote mode).
    pub fn set_local_addresses(&mut self, job_address: String, results_address: String) {
        self.local_job_address = Some(job_address);
        self.local_results_address = Some(results_address);
    }

    /// Configure remote executor addresses (for helping other coordinators when idle).
    pub fn set_remote_addresses(&mut self, job_address: String, results_address: String) {
        self.remote_job_address = Some(job_address);
        self.remote_results_address = Some(results_address);
    }

    /// Reconnect executor threads to remote job/results sockets (idle mode).
    /// Only sends RECONNECT if remote addresses are configured and executors
    /// are not already connected to remote.
    fn connect_executors_to_remote(&mut self) -> Result<()> {
        if self.executors_remote {
            return Ok(()); // already remote
        }
        if let (Some(ref job), Some(ref results)) =
            (&self.remote_job_address, &self.remote_results_address)
        {
            info!("Connecting executors to remote coordinator: jobs={job} results={results}");
            self.dispatcher.send_reconnect(job, results)?;
            self.executors_remote = true;
        }
        Ok(())
    }

    /// Reconnect executor threads back to local job/results sockets (busy mode).
    /// Only sends RECONNECT if executors are currently in remote mode.
    fn connect_executors_to_local(&mut self) -> Result<()> {
        if !self.executors_remote {
            return Ok(()); // already local
        }
        if let (Some(ref job), Some(ref results)) =
            (&self.local_job_address, &self.local_results_address)
        {
            info!("Reconnecting executors to local dispatcher: jobs={job} results={results}");
            self.dispatcher.send_reconnect(job, results)?;
            self.executors_remote = false;
            // Give executors time to process the reconnection.
            // Executors poll the control socket every 100-5000ms depending on mode.
            // This sleep is best-effort; a proper ack mechanism would be more robust
            // but significantly more complex for marginal benefit.
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        Ok(())
    }

    /// Mark that the executor has `ContextIO` configured, enabling
    /// delegation of sub-flows containing context:// functions.
    pub fn set_has_context_proxy(&mut self) {
        self.has_context_proxy = true;
    }

    /// Set the sub-flow registry shared with the executor.
    pub fn set_subflow_registry(&mut self, registry: crate::executor::SubflowRegistry) {
        self.subflow_registry = Some(registry);
    }

    /// Send a DONE signal to all connected executors, telling them to exit.
    ///
    /// # Errors
    ///
    /// Returns an error if the DONE message could not be sent.
    pub fn send_done(&mut self) -> Result<()> {
        self.dispatcher.send_done()
    }

    /// Enter a loop - waiting for a submission from the client, or disconnection of the client
    ///
    /// # Errors
    ///
    /// Returns an error if there was some issue while waiting for a submission to be sent, usually
    /// related to some networking issue, busy ports etc.
    ///
    #[cfg(feature = "submission")]
    pub fn submission_loop(&mut self, loop_forever: bool) -> Result<()> {
        let mut last_result: Result<()> = Ok(());

        loop {
            // While idle, connect executors to remote coordinator (if configured)
            if let Err(e) = self.connect_executors_to_remote() {
                info!("Could not connect to remote coordinator: {e}");
            }

            let submission = self.submission_handler.wait_for_submission()?;
            let Some(submission) = submission else {
                break;
            };

            // Reconnect executors to local dispatcher before executing
            if let Err(e) = self.connect_executors_to_local() {
                error!("Could not reconnect executors to local: {e}");
            }

            last_result = self.execute_flow(submission);
            if let Err(ref e) = last_result {
                error!("Flow execution failed: {e}");
            }
            if !loop_forever {
                break;
            }
        }

        self.submission_handler.coordinator_is_exiting(last_result)
    }

    /// Execute a sub-flow and return its `RunState`, which may contain boundary
    /// outputs (values destined for functions in the parent flow).
    ///
    /// Unlike `execute_flow`, this does not notify a submission handler and
    /// returns the `RunState` so the caller can drain boundary outputs.
    ///
    /// # Errors
    ///
    /// Returns an error if the execution did not complete normally.
    #[allow(unused_variables, unused_mut)]
    pub fn execute_subflow(&mut self, submission: Submission) -> Result<RunState> {
        self.job_timeout = submission.job_timeout;
        self.dispatcher
            .set_results_timeout(submission.job_timeout)?;
        let mut state = RunState::new(submission);
        state.set_subflow();

        #[cfg(feature = "metrics")]
        let mut metrics = Metrics::new(state.num_functions(), state.num_processes());

        state.init()?;

        self.run_jobs(
            &mut state,
            #[cfg(feature = "metrics")]
            &mut metrics,
        )?;

        Ok(state)
    }

    /// Execute a flow by looping while there are jobs to be processed.
    ///
    /// The outer loop exists for the debugger: it allows resetting all state and restarting
    /// execution from scratch. The inner dispatch/retire cycle is in [`run_jobs`](Self::run_jobs).
    ///
    /// # Errors
    ///
    /// Returns an error if the execution of the flow did not complete normally.
    #[allow(unused_variables, unused_mut)]
    pub fn execute_flow(&mut self, mut submission: Submission) -> Result<()> {
        // Handle sub-flow delegation for each flow_id/peer pair
        let flow_ids = std::mem::take(&mut submission.delegate_flow_ids);
        let peer_addrs = std::mem::take(&mut submission.peer_addresses);
        for flow_id in &flow_ids {
            let peer_addr = peer_addrs.get(flow_id).ok_or_else(|| {
                format!("Sub-flow #{flow_id} requested for delegation but no peer address assigned")
            })?;
            info!("Delegating sub-flow #{flow_id} to peer at {peer_addr}");
            self.setup_delegation(*flow_id, peer_addr, &mut submission)?;
        }

        self.job_timeout = submission.job_timeout;
        self.dispatcher
            .set_results_timeout(submission.job_timeout)?;
        let mut state = RunState::new(submission);

        #[cfg(feature = "metrics")]
        let mut metrics = Metrics::new(state.num_functions(), state.num_processes());
        #[cfg(all(feature = "metrics", feature = "debugger"))]
        {
            let num_proc = state.num_processes();
            let mut names = vec![(String::new(), String::new()); num_proc];
            for (id, func) in state.get_functions() {
                if let Some(entry) = names.get_mut(*id) {
                    *entry = (func.name().to_string(), func.route().to_string());
                }
            }
            metrics.set_function_names(names);
        }

        self.debugger_start(&state);

        // Outer loop: allows the debugger to restart execution from scratch
        let flow_result = loop {
            state.init()?;
            #[cfg(feature = "metrics")]
            {
                metrics.reset();
                TOTAL_GET_RESULT_US.store(0, std::sync::atomic::Ordering::Relaxed);
                TOTAL_RETIRE_JOB_US.store(0, std::sync::atomic::Ordering::Relaxed);
                crate::run_state::reset_retire_timers();
            }

            // If the debug client ended the session (e.g. the 'e' command), stop the outer
            // loop rather than returning immediately, so the client can still be notified
            // that execution has ended.
            let iteration_result = match self.run_jobs(
                &mut state,
                #[cfg(feature = "metrics")]
                &mut metrics,
            ) {
                Ok(result) => result,
                Err(err) if is_debugger_exit_error(&err) => break Err(err),
                Err(err) => return Err(err),
            };

            // After execution ends, give the debugger a chance to inspect final state
            // and potentially request a restart
            let restart = if matches!(iteration_result, FlowIterationResult::Restart) {
                true
            } else {
                match self.debugger_end_of_execution(
                    &mut state,
                    #[cfg(feature = "metrics")]
                    &mut metrics,
                ) {
                    Ok(restart) => restart,
                    Err(err) if is_debugger_exit_error(&err) => break Err(err),
                    Err(err) => return Err(err),
                }
            };

            if !restart {
                break Ok(());
            }
        };

        #[cfg(feature = "trace")]
        state.write_trace()?;

        // Finalize metrics and notify the submission handler that execution has ended, even
        // when the debug client ended the session early — the client needs the notification
        // to exit cleanly (e.g. flowrcli)
        #[cfg(feature = "metrics")]
        metrics.stop_timer();
        #[cfg(feature = "metrics")]
        metrics.set_jobs_created(state.get_number_of_jobs_created());
        #[cfg(all(feature = "submission", feature = "metrics"))]
        self.submission_handler
            .flow_execution_ended(&state, metrics)?;
        #[cfg(all(feature = "submission", not(feature = "metrics")))]
        self.submission_handler.flow_execution_ended(&state)?;

        flow_result
    }

    /// Extract a sub-flow from the submission, rewrite WASM URLs for remote
    /// access, and register it in the sub-flow registry for delegation to
    /// a peer coordinator.
    fn setup_delegation(
        &self,
        flow_id: usize,
        peer_addr: &str,
        submission: &mut Submission,
    ) -> Result<()> {
        let registry = self
            .subflow_registry
            .as_ref()
            .ok_or("Sub-flow delegation requested but no sub-flow registry is configured")?;
        info!("Delegating sub-flow #{flow_id}");
        let (mut extracted, input_map) = submission
            .manifest
            .delegate_subflow(flow_id)
            .map_err(|e| format!("Could not delegate sub-flow #{flow_id}: {e}"))?;

        // Check for context functions — they require proxy capability
        let context_count = extracted
            .functions()
            .values()
            .filter(|f| f.get_implementation_location().starts_with("context://"))
            .count();
        if context_count > 0 {
            if self.has_context_proxy {
                info!(
                    "Sub-flow #{flow_id} contains {context_count} context:// function(s) — \
                     will be proxied back to origin during execution"
                );
            } else {
                return Err(format!(
                    "Cannot delegate sub-flow #{flow_id}: it contains {context_count} context:// \
                     function(s) but no context proxy is configured"
                )
                .into());
            }
        }

        // Rewrite file:// WASM URLs to http:// so the peer can fetch them
        // from this coordinator's WASM HTTP server
        if let Some(ref base_url) = self.wasm_base_url {
            let rewritten = extracted.rewrite_wasm_urls(base_url);
            if rewritten > 0 {
                info!("Rewrote {rewritten} file:// WASM URL(s) to http:// for peer delegation");
            }
        } else {
            // If no WASM server is running, check whether the extracted manifest
            // contains file:// URLs that the peer would not be able to access.
            let file_count = extracted
                .functions()
                .values()
                .filter(|f| f.get_implementation_url().scheme() == "file")
                .count();
            if file_count > 0 {
                return Err(format!(
                    "Cannot delegate sub-flow #{flow_id}: it contains {file_count} file:// WASM \
                     function(s) but no WASM server is running to serve them"
                )
                .into());
            }
        }

        let subflow_url = url::Url::parse(&format!("subflow://{flow_id}"))
            .map_err(|e| format!("Invalid subflow URL: {e}"))?;
        let mut manifests = registry
            .write()
            .map_err(|_| "Could not gain write access to the sub-flow registry")?;
        info!(
            "Registered sub-flow #{flow_id} with {} functions",
            extracted.functions().len()
        );
        info!("Sub-flow #{flow_id} will be executed on remote peer at {peer_addr}");
        // Preserve the extracted manifest for possible reconstitution
        submission
            .extracted_subflows
            .insert(flow_id, extracted.clone());
        manifests.insert(
            subflow_url,
            (extracted, input_map, Some(peer_addr.to_string())),
        );
        Ok(())
    }

    /// Run the inner dispatch/retire loop until no more jobs remain or the debugger
    /// requests a restart.
    ///
    /// Returns `FlowIterationResult::Restart` if the debugger requested a reset,
    /// or `FlowIterationResult::Done` when execution finishes normally.
    #[allow(unused_variables, unused_mut)]
    fn run_jobs(
        &mut self,
        state: &mut RunState,
        #[cfg(feature = "metrics")] metrics: &mut Metrics,
    ) -> Result<FlowIterationResult> {
        if self.debugger_wait_if_enabled(state)? {
            return Ok(FlowIterationResult::Restart);
        }

        #[cfg(feature = "submission")]
        self.submission_handler.flow_execution_starting()?;

        #[cfg(feature = "metrics")]
        let mut total_dispatch_us: u128 = 0;
        #[cfg(feature = "metrics")]
        let mut total_retire_us: u128 = 0;
        #[cfg(feature = "metrics")]
        let mut loop_count: u64 = 0;

        loop {
            trace!("{state}");

            #[cfg(feature = "submission")]
            if self.submission_handler.should_stop()? {
                break;
            }

            if self.debugger_check_mid_loop(state)? {
                return Ok(FlowIterationResult::Restart);
            }

            #[cfg(feature = "metrics")]
            let dispatch_start = Instant::now();

            let action = self.dispatch_jobs(
                state,
                #[cfg(feature = "metrics")]
                metrics,
            )?;

            #[cfg(feature = "metrics")]
            {
                total_dispatch_us += dispatch_start.elapsed().as_micros();
            }

            if action.should_restart() {
                return Ok(FlowIterationResult::Restart);
            }

            #[cfg(feature = "metrics")]
            let retire_start = Instant::now();

            let action = self.retire_jobs(
                state,
                #[cfg(feature = "metrics")]
                metrics,
            )?;

            #[cfg(feature = "metrics")]
            {
                total_retire_us += retire_start.elapsed().as_micros();
                loop_count += 1;
            }

            // Send any values destined for delegated functions to the peer
            // and receive boundary outputs back into the local flow
            if state.has_delegated_functions() {
                let peer_outputs = state.drain_peer_outputs();
                if !peer_outputs.is_empty() {
                    info!("Sending {} values to peer coordinator", peer_outputs.len());
                    // Convert peer_outputs to input tuples for the peer
                    let inputs: Vec<(usize, usize, serde_json::Value)> = peer_outputs
                        .iter()
                        .map(|o| {
                            (
                                o.connection.destination_id,
                                o.connection.destination_io_number,
                                o.value.clone(),
                            )
                        })
                        .collect();

                    // TODO: submit to peer and receive boundary outputs
                    // This requires maintaining the peer connection across the
                    // dispatch/retire loop. For now, log what would be sent.
                    for (dest_id, dest_io, value) in &inputs {
                        info!("  -> #{dest_id}:{dest_io} = {value}");
                    }
                }
            }

            #[cfg(all(feature = "submission", any(feature = "metrics", feature = "debugger")))]
            self.submission_handler
                .jobs_created(state.get_number_of_jobs_created());

            if action.should_restart() {
                return Ok(FlowIterationResult::Restart);
            }

            if state.number_jobs_running() == 0 && state.number_jobs_ready() == 0 {
                break;
            }
        }

        #[cfg(feature = "metrics")]
        {
            let get_result_ms =
                TOTAL_GET_RESULT_US.load(std::sync::atomic::Ordering::Relaxed) / 1000;
            let retire_job_ms =
                TOTAL_RETIRE_JOB_US.load(std::sync::atomic::Ordering::Relaxed) / 1000;
            info!(
                "Coordinator loop: {} iterations, dispatch: {}ms, retire: {}ms \
                 (get_result: {}ms, retire_job: {}ms)",
                loop_count,
                total_dispatch_us / 1000,
                total_retire_us / 1000,
                get_result_ms,
                retire_job_ms,
            );
            crate::run_state::log_retire_breakdown();
        }

        Ok(FlowIterationResult::Done)
    }

    /// After a flow execution iteration ends (without restart), finalize metrics and
    /// give the debugger a chance to inspect final state and potentially request a restart.
    ///
    /// Returns `true` if the debugger requested a restart.
    #[allow(unused_variables, unused_mut)]
    fn debugger_end_of_execution(
        &mut self,
        state: &mut RunState,
        #[cfg(feature = "metrics")] metrics: &mut Metrics,
    ) -> Result<bool> {
        #[cfg(feature = "metrics")]
        {
            metrics.stop_timer();
            metrics.set_jobs_created(state.get_number_of_jobs_created());
            metrics.set_jobs_per_function(state.jobs_per_function());
        }

        #[cfg(feature = "debugger")]
        if state.submission.debug_enabled {
            let action = self.debugger.execution_ended(
                state,
                #[cfg(feature = "metrics")]
                Some(metrics),
            )?;
            return Ok(action.should_restart());
        }

        Ok(false)
    }

    /// Try to get a result back from an executor, using a strategy that avoids unnecessary
    /// blocking. Returns the next result tuple or `None` if there are ready jobs to dispatch.
    ///
    /// 1. First, attempt a **non-blocking** receive. If a result is already available,
    ///    return it immediately.
    /// 2. If no result is available but there are **ready jobs** waiting to be dispatched,
    ///    return `None` so the caller can dispatch them first rather than blocking here.
    ///    Those dispatched jobs may produce results faster than waiting for in-flight ones.
    /// 3. If no result is available and there are **no ready jobs** to dispatch, then
    ///    **block** waiting for the next result, since there is nothing else productive
    ///    the coordinator can do.
    #[allow(clippy::type_complexity)]
    fn get_result(
        &mut self,
        state: &RunState,
    ) -> Result<Option<(usize, String, Result<(Option<Value>, RunAgain)>)>> {
        // Step 1: Non-blocking attempt
        if let Ok(result) = self.dispatcher.get_next_result(false) {
            return Ok(Some(result));
        }

        // Step 2: If there are ready jobs, don't block — let the caller dispatch them first
        if state.number_jobs_ready() > 0 {
            return Ok(None);
        }

        // Step 3: Nothing else to do, so block waiting for the next result
        match self.dispatcher.get_next_result(true) {
            Ok(result) => Ok(Some(result)),
            Err(e) => Err(e),
        }
    }

    /// Retire as many jobs as possible, draining all available results.
    ///
    /// Processes results in a tight loop: first blocking for one result if nothing
    /// else is ready, then draining all immediately available results before
    /// returning to the dispatch/retire outer loop.
    fn retire_jobs(
        &mut self,
        state: &mut RunState,
        #[cfg(feature = "metrics")] metrics: &mut Metrics,
    ) -> Result<DebugAction> {
        if state.number_jobs_running() == 0 {
            return Ok(DebugAction::Continue);
        }

        // Check for expired jobs and re-queue them
        if self.job_timeout.is_some() {
            state.requeue_expired_jobs(
                Self::MAX_JOB_RETRIES,
                #[cfg(feature = "metrics")]
                metrics,
            )?;
        }

        // Get the first result (may block if no ready jobs to dispatch)
        let first_result = {
            #[cfg(feature = "metrics")]
            let timer = MetricsTimer::new(&TOTAL_GET_RESULT_US);
            let result = self.get_result(state);
            #[cfg(feature = "metrics")]
            drop(timer);
            result
        };

        match first_result {
            Ok(Some((job_id, executor_id, result))) => {
                #[cfg(feature = "metrics")]
                metrics.record_executor(&executor_id);
                let action = self.retire_one_job(
                    state,
                    job_id,
                    result,
                    #[cfg(feature = "metrics")]
                    metrics,
                )?;
                if action.should_restart() || action.should_display() {
                    return Ok(action);
                }
            }
            Ok(None) => return Ok(DebugAction::Continue),
            Err(err) => {
                error!("\t{err}");
                return self.debugger_error(state, &err);
            }
        }

        // Drain all immediately available results without blocking
        while state.number_jobs_running() > 0 {
            if let Ok(result) = self.dispatcher.get_next_result(false) {
                let (job_id, executor_id, job_result) = result;
                #[cfg(feature = "metrics")]
                metrics.record_executor(&executor_id);
                let action = self.retire_one_job(
                    state,
                    job_id,
                    job_result,
                    #[cfg(feature = "metrics")]
                    metrics,
                )?;

                if action.should_restart() || action.should_display() {
                    return Ok(action);
                }
            } else {
                break; // no more results immediately available
            }
        }

        Ok(DebugAction::Continue)
    }

    /// Retire a single job result
    fn retire_one_job(
        &mut self,
        state: &mut RunState,
        job_id: usize,
        result: Result<(Option<Value>, RunAgain)>,
        #[cfg(feature = "metrics")] metrics: &mut Metrics,
    ) -> Result<DebugAction> {
        #[cfg(feature = "metrics")]
        let _retire_timer = MetricsTimer::new(&TOTAL_RETIRE_JOB_US);

        let (mut action, job) = state.retire_job(
            job_id,
            result,
            #[cfg(feature = "metrics")]
            metrics,
            #[cfg(feature = "debugger")]
            &mut self.debugger,
        )?;

        self.debugger_job_done(&mut action, state, &job);

        Ok(action)
    }

    /// Dispatch as many jobs as possible for parallel execution.
    ///
    /// Returns a `DebugAction` indicating whether the debugger wants to display the
    /// next output or restart execution.
    #[allow(clippy::unnecessary_wraps)]
    fn dispatch_jobs(
        &mut self,
        state: &mut RunState,
        #[cfg(feature = "metrics")] metrics: &mut Metrics,
    ) -> Result<DebugAction> {
        let mut action = DebugAction::Continue;

        while let Some(job) = state.get_next_job() {
            match self.dispatch_a_job(
                job,
                state,
                #[cfg(feature = "metrics")]
                metrics,
            ) {
                Ok(a) => action = a,
                Err(err) => {
                    error!("Error dispatching job: {err}");
                    debug!("{state}");
                }
            }
        }

        Ok(action)
    }

    /// Dispatch a single job for execution via the dispatcher.
    /// Takes `Job` by value to avoid cloning on the success path.
    fn dispatch_a_job(
        &mut self,
        mut job: Job,
        state: &mut RunState,
        #[cfg(feature = "metrics")] metrics: &mut Metrics,
    ) -> Result<DebugAction> {
        let action = self.debugger_check_before_job(state, &job)?;

        // Rewrite file:// URLs to http:// for remote executor access
        if job.payload.implementation_url.scheme() == "file" {
            if let Some(ref base_url) = self.wasm_base_url {
                // Convert file:///path/to/file.wasm -> http://host:port/path/to/file.wasm
                let path = job.payload.implementation_url.path();
                if let Ok(http_url) = base_url.join(path) {
                    trace!(
                        "Rewriting WASM URL: {} -> {http_url}",
                        job.payload.implementation_url
                    );
                    job.payload.implementation_url = http_url;
                }
            }
        }

        self.dispatcher.send_job_for_execution(&job.payload)?;

        job.ttl = self.job_timeout.and_then(|d| Instant::now().checked_add(d));
        state.start_job(job);

        #[cfg(feature = "metrics")]
        metrics.track_max_jobs(state.number_jobs_running());

        Ok(action)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {
    use std::time::Duration;

    use serial_test::serial;

    use flowcore::model::input::Input;
    #[cfg(feature = "metrics")]
    use flowcore::model::metrics::Metrics;
    use flowcore::model::output_connection::OutputConnection;
    use flowcore::model::runtime_function::RuntimeFunction;
    use flowcore::model::submission::Submission;

    #[cfg(feature = "submission")]
    use crate::submission_handler::SubmissionHandler;

    #[cfg(feature = "debugger")]
    use crate::debug_command::DebugCommand;
    #[cfg(feature = "debugger")]
    use crate::debugger_handler::DebuggerHandler;
    #[cfg(feature = "debugger")]
    use crate::job::Job;
    #[cfg(feature = "debugger")]
    use crate::run_state::State;

    use super::Coordinator;
    use crate::dispatcher::Dispatcher;
    use crate::run_state::RunState;
    use crate::test_helper::fixtures::{get_bind_addresses, get_four_ports, test_manifest};

    #[cfg(feature = "metrics")]
    #[test]
    fn metrics_timer_accumulates() {
        use std::sync::atomic::Ordering;
        // Leak a Box to get a &'static AtomicU64 for the timer
        let counter: &'static std::sync::atomic::AtomicU64 =
            Box::leak(Box::new(std::sync::atomic::AtomicU64::new(0)));
        {
            let _timer = super::MetricsTimer::new(counter);
            std::thread::sleep(Duration::from_millis(1));
        }
        // Timer dropped — counter should have accumulated some microseconds
        assert!(
            counter.load(Ordering::Relaxed) > 0,
            "MetricsTimer should accumulate elapsed time on drop"
        );
    }

    fn test_submission(functions: Vec<RuntimeFunction>) -> Submission {
        Submission::new(
            test_manifest(functions),
            None,
            Some(Duration::from_millis(100)),
            #[cfg(feature = "debugger")]
            false,
            #[cfg(feature = "trace")]
            None,
        )
    }

    fn test_dispatcher() -> Dispatcher {
        Dispatcher::new(&get_bind_addresses(get_four_ports())).expect("Could not create dispatcher")
    }

    #[cfg(feature = "debugger")]
    struct DummyDebugServer {
        /// If true, return `ExitDebugger` (for empty flows that can't `Continue`)
        exit_immediately: bool,
    }

    #[cfg(feature = "debugger")]
    impl DebuggerHandler for DummyDebugServer {
        fn start(&mut self) {}
        fn job_breakpoint(&mut self, _job: &Job, _function: &RuntimeFunction, _states: Vec<State>) {
        }
        fn flow_unblock_breakpoint(&mut self, _flow_id: usize) {}
        fn send_breakpoint(
            &mut self,
            _: &str,
            _source_process_id: usize,
            _output_route: &str,
            _value: &serde_json::Value,
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
        fn get_command(&mut self, _state: &RunState) -> flowcore::errors::Result<DebugCommand> {
            if self.exit_immediately {
                Ok(DebugCommand::ExitDebugger)
            } else {
                Ok(DebugCommand::Continue)
            }
        }
    }

    #[cfg(feature = "submission")]
    struct DummySubmissionHandler;

    #[cfg(feature = "submission")]
    impl SubmissionHandler for DummySubmissionHandler {
        fn flow_execution_starting(&mut self) -> flowcore::errors::Result<()> {
            Ok(())
        }

        #[cfg(feature = "debugger")]
        fn should_enter_debugger(&mut self) -> flowcore::errors::Result<bool> {
            Ok(false)
        }

        fn flow_execution_ended(
            &mut self,
            _state: &RunState,
            #[cfg(feature = "metrics")] _metrics: Metrics,
        ) -> flowcore::errors::Result<()> {
            Ok(())
        }

        fn wait_for_submission(&mut self) -> flowcore::errors::Result<Option<Submission>> {
            Ok(None)
        }

        fn coordinator_is_exiting(
            &mut self,
            result: flowcore::errors::Result<()>,
        ) -> flowcore::errors::Result<()> {
            result
        }
    }

    #[test]
    #[serial]
    fn create_coordinator() {
        let dispatcher = test_dispatcher();
        #[cfg(feature = "submission")]
        let mut submission_handler = DummySubmissionHandler;
        #[cfg(feature = "debugger")]
        let mut debug_server = DummyDebugServer {
            exit_immediately: false,
        };

        let _coordinator = Coordinator::new(
            dispatcher,
            #[cfg(feature = "submission")]
            &mut submission_handler,
            #[cfg(feature = "debugger")]
            &mut debug_server,
        );
    }

    #[test]
    #[serial]
    fn execute_empty_flow() {
        let dispatcher = test_dispatcher();
        #[cfg(feature = "submission")]
        let mut submission_handler = DummySubmissionHandler;
        #[cfg(feature = "debugger")]
        let mut debug_server = DummyDebugServer {
            exit_immediately: false,
        };

        let mut coordinator = Coordinator::new(
            dispatcher,
            #[cfg(feature = "submission")]
            &mut submission_handler,
            #[cfg(feature = "debugger")]
            &mut debug_server,
        );

        let submission = test_submission(vec![]);
        let result = coordinator.execute_flow(submission);
        assert!(result.is_ok(), "Empty flow should execute successfully");
    }

    #[test]
    #[serial]
    fn execute_empty_flow_with_no_timeout() {
        let dispatcher = test_dispatcher();
        #[cfg(feature = "submission")]
        let mut submission_handler = DummySubmissionHandler;
        #[cfg(feature = "debugger")]
        let mut debug_server = DummyDebugServer {
            exit_immediately: false,
        };

        let mut coordinator = Coordinator::new(
            dispatcher,
            #[cfg(feature = "submission")]
            &mut submission_handler,
            #[cfg(feature = "debugger")]
            &mut debug_server,
        );

        let submission = Submission::new(
            test_manifest(vec![]),
            None,
            None,
            #[cfg(feature = "debugger")]
            false,
            #[cfg(feature = "trace")]
            None,
        );
        let result = coordinator.execute_flow(submission);
        assert!(
            result.is_ok(),
            "Empty flow with no timeout should execute successfully"
        );
    }

    #[test]
    #[serial]
    fn execute_empty_flow_with_max_parallel_jobs() {
        let dispatcher = test_dispatcher();
        #[cfg(feature = "submission")]
        let mut submission_handler = DummySubmissionHandler;
        #[cfg(feature = "debugger")]
        let mut debug_server = DummyDebugServer {
            exit_immediately: false,
        };

        let mut coordinator = Coordinator::new(
            dispatcher,
            #[cfg(feature = "submission")]
            &mut submission_handler,
            #[cfg(feature = "debugger")]
            &mut debug_server,
        );

        let submission = Submission::new(
            test_manifest(vec![]),
            Some(4),
            Some(Duration::from_millis(100)),
            #[cfg(feature = "debugger")]
            false,
            #[cfg(feature = "trace")]
            None,
        );
        let result = coordinator.execute_flow(submission);
        assert!(
            result.is_ok(),
            "Empty flow with max_parallel_jobs should execute successfully"
        );
    }

    #[cfg(feature = "debugger")]
    #[test]
    #[serial]
    fn execute_empty_flow_with_debugger() {
        let dispatcher = test_dispatcher();
        #[cfg(feature = "submission")]
        let mut submission_handler = DummySubmissionHandler;
        let mut debug_server = DummyDebugServer {
            exit_immediately: true,
        };

        let mut coordinator = Coordinator::new(
            dispatcher,
            #[cfg(feature = "submission")]
            &mut submission_handler,
            &mut debug_server,
        );

        let submission = Submission::new(
            test_manifest(vec![]),
            None,
            Some(Duration::from_millis(100)),
            true, // debug_enabled
            #[cfg(feature = "trace")]
            None,
        );
        // ExitDebugger causes a "Debugger Exit" error — expected for empty flows
        // since Continue loops forever when no jobs have been created
        let result = coordinator.execute_flow(submission);
        assert!(
            result.is_err(),
            "Empty flow with debugger should exit via debugger"
        );
    }

    #[cfg(feature = "submission")]
    #[test]
    #[serial]
    fn submission_loop_no_submission() {
        let dispatcher = test_dispatcher();
        let mut submission_handler = DummySubmissionHandler;
        #[cfg(feature = "debugger")]
        let mut debug_server = DummyDebugServer {
            exit_immediately: false,
        };

        let mut coordinator = Coordinator::new(
            dispatcher,
            &mut submission_handler,
            #[cfg(feature = "debugger")]
            &mut debug_server,
        );

        let result = coordinator.submission_loop(false);
        assert!(
            result.is_ok(),
            "submission_loop should return Ok when no submission is available"
        );
    }

    #[test]
    #[serial]
    fn delegate_without_registry_fails() {
        let dispatcher = test_dispatcher();
        #[cfg(feature = "submission")]
        let mut submission_handler = DummySubmissionHandler;
        #[cfg(feature = "debugger")]
        let mut debug_server = DummyDebugServer {
            exit_immediately: false,
        };

        let mut coordinator = Coordinator::new(
            dispatcher,
            #[cfg(feature = "submission")]
            &mut submission_handler,
            #[cfg(feature = "debugger")]
            &mut debug_server,
        );

        // Set peer_addresses but no subflow_registry → should fail
        let mut submission = test_submission(vec![]);
        submission.delegate_flow_ids = vec![1];
        submission
            .peer_addresses
            .insert(1, "127.0.0.1:9999".to_string());
        let result = coordinator.execute_flow(submission);
        assert!(
            result.is_err(),
            "Delegation without a subflow registry should fail"
        );
    }

    #[test]
    #[serial]
    fn delegate_without_peer_fails() {
        let dispatcher = test_dispatcher();
        #[cfg(feature = "submission")]
        let mut submission_handler = DummySubmissionHandler;
        #[cfg(feature = "debugger")]
        let mut debug_server = DummyDebugServer {
            exit_immediately: false,
        };

        let mut coordinator = Coordinator::new(
            dispatcher,
            #[cfg(feature = "submission")]
            &mut submission_handler,
            #[cfg(feature = "debugger")]
            &mut debug_server,
        );

        // Set up registry but no peer_address → delegate_flow_id is set
        // but no peer address is set, so peer_address is None
        let executor = crate::executor::Executor::new();
        coordinator.set_subflow_registry(executor.subflow_registry());

        let mut submission = test_submission(vec![]);
        submission.delegate_flow_ids = vec![1];
        submission
            .peer_addresses
            .insert(1, "127.0.0.1:9999".to_string());
        // delegate_subflow will fail because flow_id 1 doesn't exist
        // in the empty manifest
        let result = coordinator.execute_flow(submission);
        assert!(
            result.is_err(),
            "Delegation without a valid sub-flow should fail"
        );
    }

    #[test]
    #[serial]
    fn remote_reconnection_state_tracking() {
        let dispatcher = test_dispatcher();
        #[cfg(feature = "submission")]
        let mut submission_handler = DummySubmissionHandler;
        #[cfg(feature = "debugger")]
        let mut debug_server = DummyDebugServer {
            exit_immediately: false,
        };

        let mut coordinator = Coordinator::new(
            dispatcher,
            #[cfg(feature = "submission")]
            &mut submission_handler,
            #[cfg(feature = "debugger")]
            &mut debug_server,
        );

        // Initially not remote
        assert!(!coordinator.executors_remote);

        // Without remote addresses configured, connect_to_remote is a no-op
        assert!(coordinator.connect_executors_to_remote().is_ok());
        assert!(!coordinator.executors_remote);

        // Without local addresses, connect_to_local is a no-op
        assert!(coordinator.connect_executors_to_local().is_ok());
        assert!(!coordinator.executors_remote);

        // Configure remote addresses
        coordinator.set_remote_addresses(
            "tcp://10.0.0.1:5555".to_string(),
            "tcp://10.0.0.1:5556".to_string(),
        );
        assert!(coordinator.connect_executors_to_remote().is_ok());
        assert!(coordinator.executors_remote);

        // Second call is a no-op (already remote)
        assert!(coordinator.connect_executors_to_remote().is_ok());
        assert!(coordinator.executors_remote);

        // Configure local addresses and reconnect back
        coordinator.set_local_addresses(
            "tcp://127.0.0.1:6666".to_string(),
            "tcp://127.0.0.1:6667".to_string(),
        );
        assert!(coordinator.connect_executors_to_local().is_ok());
        assert!(!coordinator.executors_remote);

        // Second call is a no-op (already local)
        assert!(coordinator.connect_executors_to_local().is_ok());
        assert!(!coordinator.executors_remote);
    }

    #[test]
    #[serial]
    fn context_proxy_flag_default_false() {
        let dispatcher = test_dispatcher();
        #[cfg(feature = "submission")]
        let mut submission_handler = DummySubmissionHandler;
        #[cfg(feature = "debugger")]
        let mut debug_server = DummyDebugServer {
            exit_immediately: false,
        };

        let mut coordinator = Coordinator::new(
            dispatcher,
            #[cfg(feature = "submission")]
            &mut submission_handler,
            #[cfg(feature = "debugger")]
            &mut debug_server,
        );

        assert!(!coordinator.has_context_proxy);

        coordinator.set_has_context_proxy();
        assert!(coordinator.has_context_proxy);
    }
}

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
        while let Some(submission) = self.submission_handler.wait_for_submission()? {
            let _ = self.execute_flow(submission);
            if !loop_forever {
                break;
            }
        }

        self.submission_handler.coordinator_is_exiting(Ok(()))
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
    pub fn execute_flow(&mut self, submission: Submission) -> Result<()> {
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
}

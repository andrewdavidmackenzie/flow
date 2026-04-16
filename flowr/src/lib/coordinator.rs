#[cfg(all(not(feature = "debugger"), not(feature = "submission")))]
use std::marker::PhantomData;

use log::{debug, error, info, trace};
use serde_json::Value;

use flowcore::errors::Result;
#[cfg(feature = "metrics")]
use flowcore::model::metrics::Metrics;
use flowcore::model::submission::Submission;
use flowcore::RunAgain;

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
    #[cfg(feature = "debugger")]
    /// A `Debugger` to communicate with debug clients
    debugger: Debugger<'a>,
    #[cfg(all(not(feature = "debugger"), not(feature = "submission")))]
    _data: PhantomData<&'a Dispatcher>,
}

impl<'a> Coordinator<'a> {
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
            #[cfg(feature = "debugger")]
            debugger: Debugger::new(debug_server),
            #[cfg(all(not(feature = "debugger"), not(feature = "submission")))]
            _data: PhantomData,
        }
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

    //noinspection RsReassignImmutable
    /// Execute a flow by looping while there are jobs to be processed in an inner loop.
    /// There is an outer loop for the case when you are using the debugger, to allow entering
    /// the debugger when the flow ends and at any point resetting all the state and starting
    /// execution again from the initial state
    ///
    /// # Errors
    ///
    /// Returns an error if the execution of the flow did not complete normally.
    ///
    #[allow(unused_variables, unused_assignments, unused_mut)]
    pub fn execute_flow(&mut self, submission: Submission) -> Result<()> {
        self.dispatcher
            .set_results_timeout(submission.job_timeout)?;
        let mut state = RunState::new(submission);

        #[cfg(feature = "metrics")]
        let mut metrics = Metrics::new(state.num_functions());

        #[cfg(feature = "debugger")]
        if state.submission.debug_enabled {
            self.debugger.start();
        }

        let mut restart = false;
        let mut display_next_output = false;

        // This outer loop is just a way of restarting execution from scratch if the debugger requests it
        'flow_execution: loop {
            state.init()?;
            #[cfg(feature = "metrics")]
            metrics.reset();

            // If debugging - then prior to starting execution - enter the debugger
            #[cfg(feature = "debugger")]
            if state.submission.debug_enabled {
                (display_next_output, restart) = self.debugger.wait_for_command(&mut state)?;
            }

            #[cfg(feature = "submission")]
            self.submission_handler.flow_execution_starting()?;

            'jobs: loop {
                trace!("{state}");
                #[cfg(feature = "debugger")]
                if state.submission.debug_enabled
                    && self.submission_handler.should_enter_debugger()?
                {
                    (display_next_output, restart) = self.debugger.wait_for_command(&mut state)?;
                    if restart {
                        break 'jobs;
                    }
                }

                (display_next_output, restart) = self.dispatch_jobs(
                    &mut state,
                    #[cfg(feature = "metrics")]
                    &mut metrics,
                )?;

                if restart {
                    break 'jobs;
                }

                (display_next_output, restart) = self.retire_jobs(
                    &mut state,
                    #[cfg(feature = "metrics")]
                    &mut metrics,
                )?;

                if restart {
                    break 'jobs;
                }

                if state.number_jobs_running() == 0 && state.number_jobs_ready() == 0 {
                    // execution is done - but not returning here allows us to go into debugger
                    // at the end of execution, inspect state and possibly reset and rerun
                    break 'jobs;
                }
            } // jobs loop end

            // flow execution has ended
            #[allow(clippy::collapsible_if)]
            #[cfg(feature = "debugger")]
            if !restart {
                {
                    // If debugging then enter the debugger for a final time before ending flow execution
                    if state.submission.debug_enabled {
                        (display_next_output, restart) =
                            self.debugger.execution_ended(&mut state)?;
                    }
                }
            }

            // if no debugger then end execution always
            // if a debugger - then end execution if the debugger has not requested a restart
            if !restart {
                break 'flow_execution;
            }
        }

        #[cfg(feature = "metrics")]
        metrics.stop_timer();
        #[cfg(feature = "metrics")]
        metrics.set_jobs_created(state.get_number_of_jobs_created());
        #[cfg(all(feature = "submission", feature = "metrics"))]
        self.submission_handler
            .flow_execution_ended(&state, metrics)?;
        #[cfg(all(feature = "submission", not(feature = "metrics")))]
        self.submitter.flow_execution_ended(&state)?;

        Ok(()) // Normal flow completion exit
    }

    // Get a result back from an executor
    #[allow(clippy::type_complexity)]
    fn get_result(
        &mut self,
        state: &RunState,
    ) -> Result<Option<(usize, Result<(Option<Value>, RunAgain)>)>> {
        if let Ok(result) = self.dispatcher.get_next_result(false) {
            return Ok(Some(result));
        }

        if state.number_jobs_ready() > 0 {
            return Ok(None);
        }

        match self.dispatcher.get_next_result(true) {
            Ok(result) => Ok(Some(result)),
            Err(e) => Err(e),
        }
    }

    // Retire as many jobs as possible, based on returned results.
    // NOTE: This will block waiting for the last pending result
    fn retire_jobs(
        &mut self,
        state: &mut RunState,
        #[cfg(feature = "metrics")] metrics: &mut Metrics,
    ) -> Result<(bool, bool)> {
        let mut display_next_output = false;
        let mut restart = false;

        if state.number_jobs_running() > 0 {
            match self.get_result(state) {
                Ok(Some(result)) => {
                    let job;

                    (display_next_output, restart, job) = state.retire_a_job(
                        #[cfg(feature = "metrics")]
                        metrics,
                        result,
                        #[cfg(feature = "debugger")]
                        &mut self.debugger,
                    )?;
                    #[cfg(feature = "debugger")]
                    if display_next_output {
                        (display_next_output, restart) = self.debugger.job_done(state, &job);
                        if restart {
                            return Ok((display_next_output, restart));
                        }
                    }
                }

                Ok(None) => {
                    info!(
                        "No result was immediately available, but jobs are ready to be dispatched.\
                     So coordinator avoided blocking for result. Will be received next time around"
                    );
                }

                Err(err) => {
                    error!("\t{err}");
                    #[cfg(feature = "debugger")]
                    if state.submission.debug_enabled {
                        return self.debugger.error(state, err.to_string());
                    }
                    return Ok((display_next_output, restart));
                }
            }
        }

        Ok((display_next_output, restart))
    }

    // Dispatch as many jobs as possible for parallel execution.
    // Return if the debugger is requesting (display output, restart)
    fn dispatch_jobs(
        &mut self,
        state: &mut RunState,
        #[cfg(feature = "metrics")] metrics: &mut Metrics,
    ) -> Result<(bool, bool)> {
        let mut display_next_output = false;
        let mut restart = false;

        while let Some(job) = state.get_next_job() {
            match self.dispatch_a_job(
                job.clone(),
                state,
                #[cfg(feature = "metrics")]
                metrics,
            ) {
                Ok((display, rest)) => {
                    display_next_output = display;
                    restart = rest;
                }
                Err(err) => {
                    error!("Error sending on 'job_tx': {err}");
                    debug!("{state}");

                    #[cfg(feature = "debugger")]
                    return self.debugger.job_error(state, &job); // TODO avoid above clone() ?
                }
            }
        }

        Ok((display_next_output, restart))
    }

    // Dispatch a job for execution
    fn dispatch_a_job(
        &mut self,
        job: Job,
        state: &mut RunState,
        #[cfg(feature = "metrics")] metrics: &mut Metrics,
    ) -> Result<(bool, bool)> {
        #[cfg(not(feature = "debugger"))]
        let debug_options = (false, false);

        #[cfg(feature = "debugger")]
        let debug_options = self.debugger.check_prior_to_job(state, &job)?;

        self.dispatcher.send_job_for_execution(&job.payload)?;

        state.start_job(job);

        #[cfg(feature = "metrics")]
        metrics.track_max_jobs(state.number_jobs_running());

        Ok(debug_options)
    }
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use portpicker::pick_unused_port;
    use serial_test::serial;

    use flowcore::model::flow_manifest::FlowManifest;
    use flowcore::model::input::Input;
    use flowcore::model::metadata::MetaData;
    #[cfg(feature = "metrics")]
    use flowcore::model::metrics::Metrics;
    use flowcore::model::output_connection::OutputConnection;
    use flowcore::model::runtime_function::RuntimeFunction;
    use flowcore::model::submission::Submission;

    #[cfg(feature = "submission")]
    use crate::submission_handler::SubmissionHandler;

    #[cfg(feature = "debugger")]
    use crate::block::Block;
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

    fn get_bind_addresses(ports: (u16, u16, u16, u16)) -> (String, String, String, String) {
        (
            format!("tcp://*:{}", ports.0),
            format!("tcp://*:{}", ports.1),
            format!("tcp://*:{}", ports.2),
            format!("tcp://*:{}", ports.3),
        )
    }

    fn get_four_ports() -> (u16, u16, u16, u16) {
        (
            pick_unused_port().expect("No ports free"),
            pick_unused_port().expect("No ports free"),
            pick_unused_port().expect("No ports free"),
            pick_unused_port().expect("No ports free"),
        )
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
            Some(Duration::from_millis(100)),
            #[cfg(feature = "debugger")]
            false,
        )
    }

    fn test_dispatcher() -> Dispatcher {
        Dispatcher::new(&get_bind_addresses(get_four_ports())).expect("Could not create dispatcher")
    }

    #[cfg(feature = "debugger")]
    struct DummyDebugServer;

    #[cfg(feature = "debugger")]
    impl DebuggerHandler for DummyDebugServer {
        fn start(&mut self) {}
        fn job_breakpoint(&mut self, _job: &Job, _function: &RuntimeFunction, _states: Vec<State>) {
        }
        fn block_breakpoint(&mut self, _block: &Block) {}
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
        fn get_command(&mut self, _state: &RunState) -> flowcore::errors::Result<DebugCommand> {
            Ok(DebugCommand::Continue)
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
        let mut debug_server = DummyDebugServer;

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
        let mut debug_server = DummyDebugServer;

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
        let mut debug_server = DummyDebugServer;

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
        let mut debug_server = DummyDebugServer;

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
        );
        let result = coordinator.execute_flow(submission);
        assert!(
            result.is_ok(),
            "Empty flow with max_parallel_jobs should execute successfully"
        );
    }

    #[cfg(feature = "submission")]
    #[test]
    #[serial]
    fn submission_loop_no_submission() {
        let dispatcher = test_dispatcher();
        let mut submission_handler = DummySubmissionHandler;
        #[cfg(feature = "debugger")]
        let mut debug_server = DummyDebugServer;

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

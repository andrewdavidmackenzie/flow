#[cfg(all(not(feature = "debugger"), not(feature = "submission")))]
use std::marker::PhantomData;

use log::{debug, error, info, trace};
use serde_json::Value;

use flowcore::errors::*;
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

/// The `Coordinator` is responsible for coordinating the dispatching of jobs (consisting
/// of a set of Input values and an Implementation of a Function) for execution,
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
    _data: PhantomData<&'a Dispatcher>
}

impl<'a> Coordinator<'a> {
    /// Create a new `coordinator` with `num_threads` local executor threads
    pub fn new(
        dispatcher: Dispatcher,
        #[cfg(feature = "submission")] submitter: &'a mut dyn SubmissionHandler,
        #[cfg(feature = "debugger")] debug_server: &'a mut dyn DebuggerHandler
    ) -> Result<Self> {
        Ok(Coordinator {
            #[cfg(feature = "submission")]
            submission_handler: submitter,
            dispatcher,
            #[cfg(feature = "debugger")]
            debugger: Debugger::new(debug_server),
            #[cfg(all(not(feature = "debugger"), not(feature = "submission")))]
            _data: PhantomData
        })
    }

    /// Enter a loop - waiting for a submission from the client, or disconnection of the client
    #[cfg(feature = "submission")]
    pub fn submission_loop(
        &mut self,
        loop_forever: bool,
    ) -> Result<()> {
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
    #[allow(unused_variables, unused_assignments, unused_mut)]
    pub fn execute_flow(&mut self,
                        submission: Submission,) -> Result<()> {
        self.dispatcher.set_results_timeout(submission.job_timeout)?;
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
        'flow_execution:
        loop {
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
                trace!("{}", state);
                #[cfg(feature = "debugger")]
                if state.submission.debug_enabled && self.submission_handler.should_enter_debugger()? {
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
            } // 'jobs loop end

            // flow execution has ended
            #[allow(clippy::collapsible_if)]
            #[cfg(feature = "debugger")]
            if !restart {
                {
                    // If debugging then enter the debugger for a final time before ending flow execution
                    if state.submission.debug_enabled {
                        (display_next_output, restart) = self.debugger.execution_ended(&mut state)?;
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
        self.submission_handler.flow_execution_ended(&state, metrics)?;
        #[cfg(all(feature = "submission", not(feature = "metrics")))]
        self.submitter.flow_execution_ended(&state)?;

        Ok(()) // Normal flow completion exit
    }

    // Get a result back from an executor
    #[allow(clippy::type_complexity)]
    fn get_result(&mut self, state: &RunState) -> Result<Option<(usize, Result<(Option<Value>, RunAgain)>)>> {
        if let Ok(result) = self.dispatcher.get_next_result(false) {
            return Ok(Some(result));
        }

        if state.number_jobs_ready() > 0 {
            return Ok(None);
        }

        match self.dispatcher.get_next_result(true) {
            Ok(result) => Ok(Some(result)),
            Err(e) => Err(e)
        }
    }

    // Retire as many jobs as possible, based on returned results.
    // NOTE: This will block waiting for the last pending result
    fn retire_jobs(&mut self,
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
                        #[cfg(feature = "metrics")] metrics,
                        result,
                        #[cfg(feature = "debugger")] &mut self.debugger,
                    )?;
                    #[cfg(feature = "debugger")]
                    if display_next_output {
                        (display_next_output, restart) = self.debugger.job_done(state, &job)?;
                        if restart {
                            return Ok((display_next_output, restart));
                        }
                    }
                },

                Ok(None) => {
                    info!("No result was immediately available, but jobs are ready to be dispatched.\
                     So coordinator avoided blocking for result. Will be received next time around");
                },

                Err(err) => {
                    error!("\t{}", err.to_string());
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
                    error!("Error sending on 'job_tx': {}", err.to_string());
                    debug!("{}", state);

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
        let debug_options = self
            .debugger
            .check_prior_to_job(state, &job)?;

        self.dispatcher.send_job_for_execution(&job.payload)?;

        state.start_job(job)?;

        #[cfg(feature = "metrics")]
        metrics.track_max_jobs(state.number_jobs_running());

        Ok(debug_options)
    }
}

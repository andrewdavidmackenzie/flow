#[cfg(all(not(feature = "debugger"), not(feature = "submission")))]
use std::marker::PhantomData;

use log::{debug, error, trace};

use flowcore::errors::*;
use flowcore::model::flow_manifest::FlowManifest;
#[cfg(feature = "metrics")]
use flowcore::model::metrics::Metrics;
use flowcore::model::submission::Submission;

#[cfg(feature = "debugger")]
use crate::debugger::Debugger;
use crate::executor::Executor;
use crate::job::Job;
use crate::run_state::RunState;
#[cfg(feature = "debugger")]
use crate::server::DebuggerProtocol;
#[cfg(feature = "submission")]
use crate::server::SubmissionProtocol;

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
    submitter: &'a mut dyn SubmissionProtocol,
    /// Executor to use to get jobs executed
    executor: Executor,
    #[cfg(feature = "debugger")]
    /// A `Debugger` to communicate with debug clients
    debugger: Debugger<'a>,
    #[cfg(all(not(feature = "debugger"), not(feature = "submission")))]
    _data: PhantomData<&'a Executor>
}

impl<'a> Coordinator<'a> {
    /// Create a new `coordinator` with `num_threads` local executor threads
    pub fn new(
        #[cfg(feature = "submission")] submitter: &'a mut dyn SubmissionProtocol,
        executor: Executor,
        #[cfg(feature = "debugger")] debug_server: &'a mut dyn DebuggerProtocol
    ) -> Self {
        Coordinator {
            #[cfg(feature = "submission")]
            submitter,
            executor,
            #[cfg(feature = "debugger")]
            debugger: Debugger::new(debug_server),
            #[cfg(all(not(feature = "debugger"), not(feature = "submission")))]
            _data: PhantomData
        }
    }

    /// Enter a loop - waiting for a submission from the client, or disconnection of the client
    #[cfg(feature = "submission")]
    pub fn submission_loop(
        &mut self,
        loop_forever: bool,
    ) -> Result<()> {
        // TODO without the client and context methods currently there is no other way to send a submission
        while let Some(submission) = self.submitter.wait_for_submission()? {
            match self.executor.load_flow(&submission.manifest_url) {
                Ok(manifest) => {
                    let r = self.execute_flow(manifest, submission);
                    return self.submitter.coordinator_is_exiting(r);
                },
                Err(e) if loop_forever => error!("{}", e),
                Err(e) => {
                    return self.submitter.coordinator_is_exiting(Err(e));
                },
            }
        }

        self.submitter.coordinator_is_exiting(Ok(()))?;

        Ok(())
    }

    //noinspection RsReassignImmutable
    /// Execute a flow by looping while there are jobs to be processed in an inner loop.
    /// There is an outer loop for the case when you are using the debugger, to allow entering
    /// the debugger when the flow ends and at any point resetting all the state and starting
    /// execution again from the initial state
    #[allow(unused_variables, unused_assignments, unused_mut)]
    pub fn execute_flow(&mut self,
                        mut manifest: FlowManifest,
                        submission: Submission,) -> Result<()> {
        self.executor.set_timeout(Some(submission.job_timeout));
        let mut state = RunState::new(manifest.take_functions(), submission);

        #[cfg(feature = "metrics")]
        let mut metrics = Metrics::new(state.num_functions());

        #[cfg(feature = "debugger")]
        if state.submission.debug {
            self.debugger.start();
        }

        let mut restart = false;
        let mut display_next_output = false;

        // This outer loop is just a way of restarting execution from scratch if the debugger requests it
        'flow_execution:
        loop {
            state.init();
            #[cfg(feature = "metrics")]
            metrics.reset();

            // If debugging - then prior to starting execution - enter the debugger
            #[cfg(feature = "debugger")]
            if state.submission.debug {
                (display_next_output, restart) = self.debugger.wait_for_command(&mut state)?;
            }

            #[cfg(feature = "submission")]
            self.submitter.flow_execution_starting()?;

            'jobs: loop {
                trace!("{}", state);
                #[cfg(feature = "debugger")]
                if state.submission.debug && self.submitter.should_enter_debugger()? {
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

                if state.number_jobs_running() > 0 {
                    match self.executor.get_next_result() {
                        Ok(job) => {
                            #[cfg(feature = "debugger")]
                            if display_next_output {
                                (display_next_output, restart) =
                                    self.debugger.job_done(&mut state, &job)?;
                                if restart {
                                    break 'jobs;
                                }
                            }

                            (display_next_output, restart) = state.retire_job(
                                #[cfg(feature = "metrics")]
                                    &mut metrics,
                                &job,
                                #[cfg(feature = "debugger")]
                                    &mut self.debugger,
                            )?;
                        }

                        #[cfg(feature = "debugger")]
                        Err(err) => {
                            if state.submission.debug {
                                (display_next_output, restart) = self.debugger.error(
                                    &mut state, err.to_string())?;
                                if restart {
                                    break 'jobs;
                                }
                            }
                        }
                        #[cfg(not(feature = "debugger"))]
                        Err(e) => error!("\t{}", e.to_string()),
                    }
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
                    if state.submission.debug {
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
        metrics.set_jobs_created(state.get_number_of_jobs_created());
        #[cfg(all(feature = "submission", feature = "metrics"))]
        self.submitter.flow_execution_ended(&state, metrics)?;
        #[cfg(all(feature = "submission", not(feature = "metrics")))]
        self.submitter.flow_execution_ended(&state)?;

        Ok(()) // Normal flow completion exit
    }

    // Dispatch as many jobs as possible for parallel execution.
    // Return if the debugger is requesting (display output, restart)
    fn dispatch_jobs(
        &mut self,
        state: &mut RunState,
        #[cfg(feature = "metrics")] metrics: &mut Metrics,
    ) -> Result<(bool, bool)> {
        let mut display_output = false;
        let mut restart = false;

        while let Some(job) = state.new_job() {
            match self.dispatch_a_job(
                &job,
                #[cfg(feature = "metrics")]
                state,
                #[cfg(feature = "metrics")]
                metrics,
            ) {
                Ok((display, rest)) => {
                    display_output = display;
                    restart = rest;
                }
                Err(err) => {
                    error!("Error sending on 'job_tx': {}", err.to_string());
                    debug!("{}", state);

                    #[cfg(feature = "debugger")]
                    return self.debugger.job_error(state, &job);
                }
            }
        }

        Ok((display_output, restart))
    }

    // Dispatch a job for execution
    fn dispatch_a_job(
        &mut self,
        job: &Job,
        #[cfg(feature = "metrics")] state: &mut RunState,
        #[cfg(feature = "metrics")] metrics: &mut Metrics,
    ) -> Result<(bool, bool)> {
        #[cfg(not(feature = "debugger"))]
        let debug_options = (false, false);

        #[cfg(feature = "metrics")]
        metrics.track_max_jobs(state.number_jobs_running());

        #[cfg(feature = "debugger")]
        let debug_options = self
            .debugger
            .check_prior_to_job(state, job)?;

        self.executor.send_job_for_execution(job)?;

        Ok(debug_options)
    }
}

#[cfg(test)]
mod test {
    use url::Url;

    use crate::coordinator::Submission;

    #[test]
    fn create_submission() {
        let manifest_url = Url::parse("file:///temp/fake/flow.toml").expect("Could not create Url");
        let _ = Submission::new(
            &manifest_url,
            Some(1),
            #[cfg(feature = "debugger")]
            false,
        );
    }
}

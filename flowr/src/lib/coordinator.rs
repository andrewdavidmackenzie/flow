use std::sync::{Arc, Mutex};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};

use log::{debug, error, info, trace};

use flowcore::lib_provider::MetaProvider;
use flowcore::model::flow_manifest::FlowManifest;
#[cfg(feature = "metrics")]
use flowcore::model::metrics::Metrics;
use flowcore::model::submission::Submission;

use crate::debugger::Debugger;
use crate::errors::*;
use crate::execution;
use crate::job::Job;
use crate::loader::Loader;
use crate::run_state::RunState;
use crate::server::{DebugServer, Server};

/// The `Coordinator` is responsible for coordinating the dispatching of jobs (consisting
/// of a set of Input values and an Implementation of a Function) for execution,
/// gathering the resulting Outputs and distributing output values to other connected function's
/// Inputs.
///
/// It accepts Flows to be executed in the form of a `Submission` struct that has the required
/// information to execute the flow.
///

pub struct Coordinator<'a> {
    /// A channel used to send Jobs out for execution
    job_tx: Sender<Job>,
    /// A channel used to receive Jobs back after execution (now including the job's output)
    job_rx: Receiver<Job>,
    /// A `Server` to communicate with clients
    server: &'a mut dyn Server,
    #[cfg(feature = "debugger")]
    /// A `Debugger` to communicate with debug clients
    debugger: Debugger<'a>,
}

impl<'a> Coordinator<'a> {
    /// Create a new `coordinator` with `num_threads` executor threads
    pub fn new(num_threads: usize, server: &'a mut dyn Server,
               #[cfg(feature = "debugger")] debug_server: &'a mut dyn DebugServer
    ) -> Self {
        let (job_tx, job_rx) = mpsc::channel();
        let (output_tx, output_rx) = mpsc::channel();

        execution::set_panic_hook();

        info!("Starting {} executor threads", num_threads);
        let shared_job_receiver = Arc::new(Mutex::new(job_rx));
        execution::start_executors(num_threads, &shared_job_receiver, &output_tx);

        #[cfg(feature = "debugger")] let debugger = Debugger::new(debug_server);

        Coordinator {
            job_tx,
            job_rx: output_rx,
            server,
            #[cfg(feature = "debugger")]
            debugger,
        }
    }

    /// Enter a loop - waiting for a submission from the client, or disconnection of the client
    pub fn submission_loop(
        &mut self,
        mut loader: Loader,
        provider: MetaProvider,
        loop_forever: bool,
    ) -> Result<()> {

        while let Some(submission) = self.server.wait_for_submission()? {
            match loader.load_flow(&provider, &submission.manifest_url) {
                Ok(manifest) => {
                    if self.execute_flow(manifest, submission)? {
                        break;
                    }
                }
                Err(e) => {
                    error!("{}", e);

                    if !loop_forever {
                        debug!("Coordinator exiting submission loop due to error");
                        bail!("{}", e);
                    }
                },
            }
        }

        Ok(())
    }

    //noinspection RsReassignImmutable
    /// Execute a flow by looping while there are jobs to be processed in an inner loop.
    /// There is an outer loop for the case when you are using the debugger, to allow entering
    /// the debugger when the flow ends and at any point resetting all the state and starting
    /// execution again from the initial state
    pub fn execute_flow(&mut self,
                        mut manifest: FlowManifest,
                        submission: Submission,) -> Result<bool> {
        let mut state = RunState::new(manifest.get_functions(), submission);

        #[cfg(feature = "metrics")]
        let mut metrics = Metrics::new(state.num_functions());

        #[cfg(feature = "debugger")]
        if state.debug {
            self.debugger.start();
        }

        #[cfg(feature = "debugger")]
        let (mut display_next_output, mut restart, mut exit_debugger);
        restart = false;
        display_next_output = false;

        // This outer loop is just a way of restarting execution from scratch if the debugger requests it
        'flow_execution: loop {
            state.init();
            #[cfg(feature = "metrics")]
            metrics.reset();

            // If debugging then check if we should enter the debugger
            #[cfg(feature = "debugger")]
            if state.debug {
                (_, _, exit_debugger) = self.debugger.wait_for_command(&state);
                if exit_debugger {
                    return Ok(true); // User requested via debugger to exit execution
                }
            }

            self.server.flow_starting()?;

            'jobs: loop {
                trace!("{}", state);
                #[cfg(feature = "debugger")]
                if state.debug && self.server.should_enter_debugger()? && self.debugger.wait_for_command(&state).2 {
                    return Ok(true); // User requested via debugger to exit execution
                }

                (_, _, exit_debugger) = self.send_jobs(
                    &mut state,
                    #[cfg(feature = "metrics")]
                    &mut metrics,
                );

                if exit_debugger {
                    return Ok(true); // User requested via debugger to exit execution
                }

                #[cfg(feature = "debugger")]
                {
                    // If debugger request it, exit the inner job loop which will cause us to reset state
                    // and restart execution, in the outer flow_execution loop
                    if restart {
                        break 'jobs;
                    }
                }

                if state.number_jobs_running() > 0 {
                    match self.job_rx.recv_timeout(state.job_timeout) {
                        Ok(job) => {
                            #[cfg(feature = "debugger")]
                            if display_next_output {
                                self.debugger.job_completed(&state, &job);
                            }

                            state.complete_job(
                                #[cfg(feature = "metrics")]
                                    &mut metrics,
                                &job,
                                #[cfg(feature = "debugger")]
                                    &mut self.debugger,
                            );
                        }

                        #[cfg(feature = "debugger")]
                        Err(err) => {
                            if state.debug {
                                self.debugger
                                    .panic(&state, format!("Error in job reception: '{}'", err));
                            }
                        }
                        #[cfg(not(feature = "debugger"))]
                        Err(e) => error!("\tError in Job reception: {}", e),
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
                    if state.debug {
                        (display_next_output, restart, exit_debugger) = self.debugger.execution_ended(&state);
                        if exit_debugger {
                            return Ok(true); // Exit debugger
                        }
                    }
                }

                // if the debugger has not requested a restart of the flow
                if !restart {
                    break 'flow_execution;
                }
            }
        }

        #[cfg(feature = "metrics")]
        metrics.set_jobs_created(state.jobs_created());
        #[cfg(feature = "metrics")]
        self.server.flow_ended(&state, metrics)?;
        #[cfg(not(feature = "metrics"))]
        self.server.flow_ended()?;

        Ok(false) // Normal flow completion exit
    }

    // Send as many jobs as possible for parallel execution.
    // Return 'true' if the debugger is requesting a restart
    fn send_jobs(
        &mut self,
        state: &mut RunState,
        #[cfg(feature = "metrics")] metrics: &mut Metrics,
    ) -> (bool, bool, bool) {
        let mut display_output = false;
        let mut restart = false;
        let mut abort = false;

        while let Some(job) = state.next_job() {
            match self.send_job(
                &job,
                state,
                #[cfg(feature = "metrics")]
                metrics,
            ) {
                Ok((display, rest, leave)) => {
                    display_output = display;
                    restart = rest;
                    abort = leave;
                }
                Err(err) => {
                    error!("Error sending on 'job_tx': {}", err.to_string());
                    debug!("{}", state);

                    #[cfg(feature = "debugger")]
                    self.debugger.job_error(state, &job);
                }
            }
        }

        (display_output, restart, abort)
    }

    // Send a job for execution
    fn send_job(
        &mut self,
        job: &Job,
        state: &mut RunState,
        #[cfg(feature = "metrics")] metrics: &mut Metrics,
    ) -> Result<(bool, bool, bool)> {
        #[cfg(not(feature = "debugger"))]
        let debug_options = (false, false, false);

        state.start(job);
        #[cfg(feature = "metrics")]
        metrics.track_max_jobs(state.number_jobs_running());

        #[cfg(feature = "debugger")]
        let debug_options = self
            .debugger
            .check_prior_to_job(state, job.job_id, job.function_id);

        // Jobs maybe sent to remote nodes over network so have to be self--contained - clone OK
        self.job_tx
            .send(job.clone())
            .chain_err(|| "Sending of job for execution failed")?;
        debug!("Job #{}:\tSent for execution", job.job_id);

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
            1,
            #[cfg(feature = "debugger")]
            false,
        );
    }
}

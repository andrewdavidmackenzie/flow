use std::marker::PhantomData;
use std::sync::{Arc, Mutex};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};

use log::{debug, error, info, trace};

use flowcore::errors::*;
use flowcore::meta_provider::MetaProvider;
use flowcore::model::flow_manifest::FlowManifest;
#[cfg(feature = "metrics")]
use flowcore::model::metrics::Metrics;
use flowcore::model::submission::Submission;

#[cfg(feature = "debugger")]
use crate::debugger::Debugger;
use crate::execution;
use crate::job::Job;
use crate::loader::Loader;
use crate::run_state::RunState;
#[cfg(feature = "debugger")]
use crate::server::DebugServer;
#[cfg(feature = "context")]
use crate::server::Server;

/// The `Coordinator` is responsible for coordinating the dispatching of jobs (consisting
/// of a set of Input values and an Implementation of a Function) for execution,
/// gathering the resulting Outputs and distributing output values to other connected function's
/// Inputs.
///
/// It accepts Flows to be executed in the form of a `Submission` struct that has the required
/// information to execute the flow.
pub struct Coordinator<'a> {
    /// A channel used to send Jobs out for execution
    job_tx: Sender<Job>,
    /// A channel used to receive Jobs back after execution (now including the job's output)
    job_rx: Receiver<Job>,
    /// A `Server` to communicate with clients
    #[cfg(feature = "context")]
    server: &'a mut dyn Server,
    #[cfg(feature = "debugger")]
    /// A `Debugger` to communicate with debug clients
    debugger: Debugger<'a>,
    phantom: PhantomData<&'a Job>,
}

impl<'a> Coordinator<'a> {
    /// Create a new `coordinator` with `num_threads` executor threads
    pub fn new(num_threads: usize,
               #[cfg(feature = "context")] server: &'a mut dyn Server,
               #[cfg(feature = "debugger")] debug_server: &'a mut dyn DebugServer
    ) -> Self {
        let (job_tx, job_rx) = mpsc::channel();
        let (output_tx, output_rx) = mpsc::channel();

        execution::set_panic_hook();

        info!("Starting {} executor threads", num_threads);
        let shared_job_receiver = Arc::new(Mutex::new(job_rx));
        execution::start_executors(num_threads, &shared_job_receiver, &output_tx);

        Coordinator {
            job_tx,
            job_rx: output_rx,
            #[cfg(feature = "context")]
            server,
            #[cfg(feature = "debugger")]
            debugger: Debugger::new(debug_server),
            phantom: PhantomData
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
                    let r = self.execute_flow(manifest, submission);
                    return self.server.server_exiting(r);
                },
                Err(e) if loop_forever => error!("{}", e),
                Err(e) => {
                    return self.server.server_exiting(Err(e));
                },
            }
        }

        self.server.server_exiting(Ok(()))
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
        let mut state = RunState::new(manifest.get_functions(), submission);

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

            self.server.flow_starting()?;

            'jobs: loop {
                trace!("{}", state);
                #[cfg(feature = "debugger")]
                if state.submission.debug && self.server.should_enter_debugger()? {
                    (display_next_output, restart) = self.debugger.wait_for_command(&mut state)?;
                    if restart {
                        break 'jobs;
                    }
                }

                (display_next_output, restart) = self.send_jobs(
                    &mut state,
                    #[cfg(feature = "metrics")]
                    &mut metrics,
                )?;

                if restart {
                    break 'jobs;
                }

                if state.number_jobs_running() > 0 {
                    match self.job_rx.recv_timeout(state.submission.job_timeout) {
                        Ok(job) => {
                            #[cfg(feature = "debugger")]
                            if display_next_output {
                                (display_next_output, restart) =
                                    self.debugger.job_completed(&mut state, &job)?;
                                if restart {
                                    break 'jobs;
                                }
                            }

                            (display_next_output, restart) = state.complete_job(
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
                                (display_next_output, restart) = self.debugger
                                    .panic(&mut state, format!("Error in job reception: '{}'", err))?;
                                if restart {
                                    break 'jobs;
                                }
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
        #[cfg(feature = "metrics")]
        self.server.flow_ended(&state, metrics)?;
        #[cfg(not(feature = "metrics"))]
        self.server.flow_ended()?;

        Ok(()) // Normal flow completion exit
    }

    // Send as many jobs as possible for parallel execution.
    // Return 'true' if the debugger is requesting a restart
    fn send_jobs(
        &mut self,
        state: &mut RunState,
        #[cfg(feature = "metrics")] metrics: &mut Metrics,
    ) -> Result<(bool, bool)> {
        let mut display_output = false;
        let mut restart = false;

        while let Some(job) = state.next_job() {
            match self.send_job(
                &job,
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

    // Send a job for execution
    fn send_job(
        &mut self,
        job: &Job,
        state: &mut RunState,
        #[cfg(feature = "metrics")] metrics: &mut Metrics,
    ) -> Result<(bool, bool)> {
        #[cfg(not(feature = "debugger"))]
        let debug_options = (false, false);

        state.start(job);
        #[cfg(feature = "metrics")]
        metrics.track_max_jobs(state.number_jobs_running());

        #[cfg(feature = "debugger")]
        let debug_options = self
            .debugger
            .check_prior_to_job(state, job)?;

        // Jobs maybe sent to remote nodes over network so have to be self--contained - clone OK
        self.job_tx
            .send(job.clone())
            .chain_err(|| "Sending of job for execution failed")?;

        trace!(
            "Job #{}: Sent for Execution of Function #{}",
            job.job_id,
            job.function_id
        );

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

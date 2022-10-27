use std::time::Duration;

use log::{debug, trace};

use flowcore::errors::*;

use crate::job::Job;

//const JOB_SOURCE_NAME: &str  = "inproc://job-source";
pub(crate) const JOB_SOURCE_NAME: &str  = "tcp://127.0.0.1:3456";

//const RESULTS_SINK_NAME: &str  = "inproc://results-sink";
pub(crate) const RESULTS_SINK_NAME: &str  = "tcp://127.0.0.1:3457";


/// `Dispatcher` structure holds information required to send jobs for execution and receive results back
pub struct Dispatcher {
    #[allow(dead_code)]
    // Context for message queues for jobs and results
    context: zmq::Context,
    // A source of jobs to be processed
    job_source: zmq::Socket,
    // A sink where to send jobs (with results)
    results_sink: zmq::Socket,
}

/// `Dispatcher` struct takes care of ending jobs for execution and receiving results
impl Dispatcher {
    /// Create a new `Executor`
    pub fn new() -> Result<Self> {
        let context = zmq::Context::new();
        let job_source = context.socket(zmq::PUSH)
            .map_err(|_| "Could not create job source socket")?;
        job_source.bind(JOB_SOURCE_NAME)
            .map_err(|_| "Could not bind to job-source socket")?;

        let results_sink = context.socket(zmq::PULL)
            .map_err(|_| "Could not create results sink socket")?;
        results_sink.bind(RESULTS_SINK_NAME)
            .map_err(|_| "Could not bind to results-sink socket")?;

        Ok(Dispatcher {
            context,
            job_source,
            results_sink,
        })
    }

    /// Set the timeout to use when waiting for job results
    /// Setting to `None` will disable timeouts and block forever
    pub fn set_results_timeout(&mut self, timeout: Option<Duration>) -> Result<()> {
        match timeout {
            Some(time) => {
                debug!("Setting results timeout to: {}ms", time.as_millis());
                self.results_sink.set_rcvtimeo(time.as_millis() as i32)
            },
            None => {
                debug!("Disabling results timeout");
                self.results_sink.set_rcvtimeo(-1)
            },
        }.map_err(|e| format!("Error setting results timeout: {e}").into())
    }

    /// Wait for, then return the next Job with results returned from executors
    pub fn get_next_result(&mut self) -> Result<Job> {
        let msg = self.results_sink.recv_msg(0)
            .map_err(|_| "Error receiving result")?;
        let message_string = msg.as_str().ok_or("Could not get message as str")?;
        serde_json::from_str(message_string)
            .map_err(|_| "Could not Deserialize Job from zmq message string".into())
    }

    // Send a `Job` for execution to executors
    pub(crate) fn send_job_for_execution(&mut self, job: &Job) -> Result<()> {
        self.job_source.send(serde_json::to_string(job)?.as_bytes(), 0)
            .map_err(|_| "Could not send Job for execution")?;

        trace!(
            "Job #{}: Sent for execution of Function #{}",
            job.job_id,
            job.function_id
        );

        Ok(())
    }
}

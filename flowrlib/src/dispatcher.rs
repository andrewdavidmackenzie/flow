use std::time::Duration;

use log::{debug, trace};

use flowcore::errors::*;

use crate::job::Job;

/// `Dispatcher` structure holds information required to send jobs for execution and receive results back
pub struct Dispatcher {
    // A source of jobs to be executed for context:// functions
    context_job_source: zmq::Socket,
    // A source of other (non-context) jobs to be executed
    job_source: zmq::Socket,
    // A sink where to send jobs (with results)
    results_sink: zmq::Socket,
}

/// `Dispatcher` struct takes care of ending jobs for execution and receiving results
impl Dispatcher {
    /// Create a new `Dispatcher` of `Job`s
    pub fn new(job_source_name: &str, context_job_source_name: &str, results_sink_name: &str) -> Result<Self> {
        let context = zmq::Context::new();
        let job_source = context.socket(zmq::PUSH)
            .map_err(|_| "Could not create job source socket")?;
        job_source.bind(job_source_name)
            .map_err(|_| "Could not bind to job socket")?;

        let context_job_source = context.socket(zmq::PUSH)
            .map_err(|_| "Could not create context job source socket")?;
        context_job_source.bind(context_job_source_name)
            .map_err(|_| "Could not bind to context job socket")?;

        let results_sink = context.socket(zmq::PULL)
            .map_err(|_| "Could not create results sink socket")?;
        results_sink.bind(results_sink_name)
            .map_err(|_| "Could not bind to results socket")?;

        Ok(Dispatcher {
            job_source,
            context_job_source,
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
        if job.implementation_url.scheme() == "context" {
            self.context_job_source.send(serde_json::to_string(job)?.as_bytes(), 0)
                .map_err(|_| "Could not send context Job for execution")?;
        } else {
            self.job_source.send(serde_json::to_string(job)?.as_bytes(), 0)
                .map_err(|_| "Could not send Job for execution")?;
        }

        trace!(
            "Job #{}: Sent for execution of Function #{}",
            job.job_id,
            job.function_id
        );

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use url::Url;
    use std::time::Duration;
    use serial_test::serial;
    use crate::job::Job;
    use portpicker::pick_unused_port;

    fn job_source_name() -> String {
        format!("tcp://127.0.0.1:{}", pick_unused_port().expect("Could not get unused port"))
    }

    fn context_job_source_name() -> String {
        format!("tcp://127.0.0.1:{}", pick_unused_port().expect("Could not get unused port"))
    }

    fn results_sink_name() -> String {
        format!("tcp://127.0.0.1:{}", pick_unused_port().expect("Could not get unused port"))
    }

    #[test]
    #[serial]
    fn test_constructor() {
        assert!(super::Dispatcher::new(
            &job_source_name(),
            &context_job_source_name(),
            &results_sink_name(),
        ).is_ok());
    }

    #[test]
    #[serial]
    fn set_timeout_to_none() {
        let mut dispatcher = super::Dispatcher::new(
            &job_source_name(),
            &context_job_source_name(),
            &results_sink_name(),
        ).expect("Could not create dispatcher");
        assert!(dispatcher.set_results_timeout(None).is_ok());
    }

    #[test]
    #[serial]
    fn set_timeout() {
        let mut dispatcher = super::Dispatcher::new(
            &job_source_name(),
            &context_job_source_name(),
            &results_sink_name(),
        ).expect("Could not create dispatcher");
        assert!(dispatcher.set_results_timeout(Some(Duration::from_millis(10))).is_ok());
    }

    #[test]
    #[serial]
    fn send_lib_job() {
        let job = Job {
            job_id: 0,
            function_id: 1,
            flow_id: 0,
            input_set: vec![],
            connections: vec![],
            implementation_url: Url::parse("lib://flowstdlib/math/add").expect("Could not parse Url"),
            result: Ok((None, false)),
        };

        let job_source_name = job_source_name();

        let mut dispatcher = super::Dispatcher::new(
            &job_source_name,
            &context_job_source_name(),
            &results_sink_name(),
        ).expect("Could not create dispatcher");

        let context = zmq::Context::new();
        let job_source = context.socket(zmq::PULL)
            .expect("Could not create PULL end of job-source socket");
        job_source.connect(&job_source_name)
            .expect("Could not bind to PULL end of job-source socket");

        assert!(dispatcher.send_job_for_execution(&job).is_ok());
    }

    #[test]
    #[serial]
    fn send_context_job() {
        let job = Job {
            job_id: 0,
            function_id: 1,
            flow_id: 0,
            input_set: vec![],
            connections: vec![],
            implementation_url: Url::parse("context://stdio/stdout").expect("Could not parse Url"),
            result: Ok((None, false)),
        };

        let context_job_source_name = context_job_source_name();

        let mut dispatcher = super::Dispatcher::new(
            &job_source_name(),
            &context_job_source_name,
            &results_sink_name(),
        ).expect("Could not create dispatcher");

        let context = zmq::Context::new();
        let context_job_source = context.socket(zmq::PULL)
            .expect("Could not create PULL end of context-job-source socket");
        context_job_source.connect(&context_job_source_name)
            .expect("Could not bind to PULL end of job-source socket");

        assert!(dispatcher.send_job_for_execution(&job).is_ok());
    }

    #[test]
    #[serial]
    fn get_job() {
        let job = Job {
            job_id: 0,
            function_id: 1,
            flow_id: 0,
            input_set: vec![],
            connections: vec![],
            implementation_url: Url::parse("context://stdio/stdout").expect("Could not parse Url"),
            result: Ok((None, false)),
        };

        let results_name = &results_sink_name();

        let mut dispatcher = super::Dispatcher::new(
            &job_source_name(),
            &context_job_source_name(),
            results_name,
        ).expect("Could not create dispatcher");

        let context = zmq::Context::new();
        let results_sink = context.socket(zmq::PUSH)
            .expect("Could not createPUSH end of results-sink socket");
        results_sink.connect(results_name)
            .expect("Could not connect to PULL end of results-sink socket");
        results_sink.send(serde_json::to_string(&job).expect("Could not convert to serde")
                              .as_bytes(), 0).expect("Could not send result of Job");

        assert!(dispatcher.get_next_result().is_ok());
    }
}

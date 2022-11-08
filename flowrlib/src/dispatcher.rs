use std::time::Duration;

use log::{debug, trace, error};
use zmq::DONTWAIT;

use flowcore::errors::*;

use crate::job::Job;

/// `Dispatcher` structure holds information required to send jobs for execution and receive results back
pub struct Dispatcher {
    // A source of lib jobs to be executed
    lib_job_socket: zmq::Socket,
    // A source of jobs to be executed for context:// and provided functions
    general_job_socket: zmq::Socket,
    // A sink where to send jobs (with results)
    results_socket: zmq::Socket,
    // a socket to send control information to subscribing executors
    control_socket: zmq::Socket,
}

/// `Dispatcher` struct takes care of ending jobs for execution and receiving results
impl Dispatcher {
    /// Create a new `Dispatcher` of `Job`s using three addresses of job queues
    pub fn new(job_queues: (String, String, String, String)) -> Result<Self> {
        let context = zmq::Context::new();
        let lib_job_socket = context.socket(zmq::PUSH)
            .map_err(|_| "Could not create job socket")?;
        lib_job_socket.bind(&job_queues.0)
            .map_err(|_| "Could not bind to job socket")?;

        let general_job_socket = context.socket(zmq::PUSH)
            .map_err(|_| "Could not create context job socket")?;
        general_job_socket.bind(&job_queues.1)
            .map_err(|_| "Could not bind to context job socket")?;

        let results_socket = context.socket(zmq::PULL)
            .map_err(|_| "Could not create results socket")?;
        results_socket.bind(&job_queues.2)
            .map_err(|_| "Could not bind to results socket")?;

        let control_socket = context.socket(zmq::PUB)
            .map_err(|_| "Could not create control socket")?;
        control_socket.bind(&job_queues.3)
            .map_err(|_| "Could not bind to control socket")?;

        Ok(Dispatcher {
            lib_job_socket,
            general_job_socket,
            results_socket,
            control_socket
        })
    }

    // Set the timeout to use when waiting for job results
    // Setting to `None` will disable timeouts and block forever
    pub(crate) fn set_results_timeout(&mut self, timeout: Option<Duration>) -> Result<()> {
        match timeout {
            Some(time) => {
                debug!("Setting results timeout to: {}ms", time.as_millis());
                self.results_socket.set_rcvtimeo(time.as_millis() as i32)
            },
            None => {
                debug!("Disabling results timeout");
                self.results_socket.set_rcvtimeo(-1)
            },
        }.map_err(|e| format!("Error setting results timeout: {e}").into())
    }

    // Wait for, then return the next Job with results returned from executors
    pub(crate) fn get_next_result(&mut self) -> Result<Job> {
        let msg = self.results_socket.recv_msg(0)
            .map_err(|_| "Error receiving result")?;
        let message_string = msg.as_str().ok_or("Could not get message as str")?;
        serde_json::from_str(message_string)
            .map_err(|_| "Could not Deserialize Job from zmq message string".into())
    }

    // Send a `Job` for execution to executors
    pub(crate) fn send_job_for_execution(&mut self, job: &Job) -> Result<()> {
        if job.implementation_url.scheme() == "lib" {
            self.lib_job_socket.send(serde_json::to_string(job)?.as_bytes(), 0)
                .map_err(|e| format!("Could not send context Job for execution: {e}"))?;
        } else {
            self.general_job_socket.send(serde_json::to_string(job)?.as_bytes(), 0)
                .map_err(|e| format!("Could not send Job for execution: {e}"))?;
        }

        trace!(
            "Job #{}: Sent for execution of Function #{}",
            job.job_id,
            job.function_id
        );

        Ok(())
    }

    /// Send a "DONE"" message to subscribed executors on the control_socket
    pub fn send_done(&mut self) -> Result<()> {
        debug!("Dispatcher announcing DONE");
        self.control_socket.send("DONE".as_bytes(), DONTWAIT)
            .chain_err(|| "Could not send 'DONE' message")
    }
}

impl Drop for Dispatcher {
    fn drop(&mut self) {
        if let Err(e) = self.send_done() {
            error!("Error while sending KILL while dropping Dispatcher: {e}");
        }
    }
}

#[cfg(test)]
mod test {
    use url::Url;
    use std::time::Duration;
    use serial_test::serial;
    use crate::job::Job;
    use portpicker::pick_unused_port;

    fn get_bind_addresses(ports: (u16, u16, u16, u16)) -> (String, String, String, String) {
        (
            format!("tcp://*:{}", ports.0),
            format!("tcp://*:{}", ports.1),
            format!("tcp://*:{}", ports.2),
            format!("tcp://*:{}", ports.3),
        )
    }

    fn get_four_ports() -> (u16, u16, u16, u16) {
        (pick_unused_port().expect("No ports free"),
            pick_unused_port().expect("No ports free"),
            pick_unused_port().expect("No ports free"),
            pick_unused_port().expect("No ports free"),
        )
    }

    #[test]
    #[serial]
    fn test_constructor() {
        let dispatcher = super::Dispatcher::new(
            get_bind_addresses(get_four_ports()));
        assert!(dispatcher.is_ok());
    }

    #[test]
    #[serial]
    fn set_timeout_to_none() {
        let mut dispatcher = super::Dispatcher::new(
            get_bind_addresses(get_four_ports())
        ).expect("Could not create dispatcher");
        assert!(dispatcher.set_results_timeout(None).is_ok());
    }

    #[test]
    #[serial]
    fn set_timeout() {
        let mut dispatcher = super::Dispatcher::new(
            get_bind_addresses(get_four_ports())
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

        let ports = get_four_ports();
        let mut dispatcher = super::Dispatcher::new(
            get_bind_addresses(ports)
        ).expect("Could not create dispatcher");

        let context = zmq::Context::new();
        let job_source = context.socket(zmq::PULL)
            .expect("Could not create PULL end of job socket");
        job_source.connect(&format!("tcp://127.0.0.1:{}", ports.0))
            .expect("Could not bind to PULL end of job socket");

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

        let ports = get_four_ports();
        let mut dispatcher = super::Dispatcher::new(
            get_bind_addresses(ports)
        ).expect("Could not create dispatcher");

        let context = zmq::Context::new();
        let context_job_source = context.socket(zmq::PULL)
            .expect("Could not create PULL end of context-job socket");
        context_job_source.connect(&format!("tcp://127.0.0.1:{}", ports.1))
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

        let ports = get_four_ports();
        let mut dispatcher = super::Dispatcher::new(
            get_bind_addresses(ports)
        ).expect("Could not create dispatcher");

        let context = zmq::Context::new();
        let results_sink = context.socket(zmq::PUSH)
            .expect("Could not create PUSH end of results socket");
        results_sink.connect(&format!("tcp://127.0.0.1:{}", ports.2))
            .expect("Could not connect to PULL end of results socket");
        results_sink.send(serde_json::to_string(&job).expect("Could not convert to serde")
                              .as_bytes(), 0).expect("Could not send result of Job");

        assert!(dispatcher.get_next_result().is_ok());
    }
}

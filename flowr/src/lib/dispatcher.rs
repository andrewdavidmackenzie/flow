use std::time::Duration;

use log::{debug, error, trace};
use serde_json::Value;
use zmq::DONTWAIT;

use flowcore::errors::{Result, ResultExt};
use flowcore::RunAgain;

use crate::job::Payload;

const WAIT: i32 = 0;

/// `Dispatcher` structure holds information required to send jobs for execution and receive results back
#[allow(clippy::struct_field_names)]
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
    ///
    /// # Errors
    ///
    /// Returns an error if the zmq sockets used to send messages between client and coordinator
    /// cannot be bound.
    ///
    pub fn new(job_queues: &(String, String, String, String)) -> Result<Self> {
        let context = zmq::Context::new();
        let lib_job_socket = context
            .socket(zmq::PUSH)
            .map_err(|_| "Could not create job socket")?;
        lib_job_socket
            .set_linger(0)
            .map_err(|_| "Could not set linger on job socket")?;
        lib_job_socket
            .bind(&job_queues.0)
            .map_err(|_| "Could not bind to job socket")?;

        let general_job_socket = context
            .socket(zmq::PUSH)
            .map_err(|_| "Could not create context job socket")?;
        general_job_socket
            .set_linger(0)
            .map_err(|_| "Could not set linger on context job socket")?;
        general_job_socket
            .bind(&job_queues.1)
            .map_err(|_| "Could not bind to context job socket")?;

        let results_socket = context
            .socket(zmq::PULL)
            .map_err(|_| "Could not create results socket")?;
        results_socket
            .set_linger(0)
            .map_err(|_| "Could not set linger on results socket")?;
        results_socket
            .bind(&job_queues.2)
            .map_err(|_| "Could not bind to results socket")?;

        let control_socket = context
            .socket(zmq::PUB)
            .map_err(|_| "Could not create control socket")?;
        control_socket
            .set_linger(0)
            .map_err(|_| "Could not set linger on control socket")?;
        control_socket
            .bind(&job_queues.3)
            .map_err(|_| "Could not bind to control socket")?;

        Ok(Dispatcher {
            lib_job_socket,
            general_job_socket,
            results_socket,
            control_socket,
        })
    }

    // Set the timeout to use when waiting for job results
    // Setting to `None` will disable timeouts and block forever
    pub(crate) fn set_results_timeout(&mut self, timeout: Option<Duration>) -> Result<()> {
        #[allow(clippy::single_match_else)]
        match timeout {
            Some(time) => {
                debug!("Setting results timeout to: {}ms", time.as_millis());
                //assert!(time.as_millis() < i32::MAX, "Truncation");
                self.results_socket
                    .set_rcvtimeo(i32::try_from(time.as_millis())?)
            }
            None => {
                debug!("Disabling results timeout");
                self.results_socket.set_rcvtimeo(-1)
            }
        }
        .map_err(|e| format!("Error setting results timeout: {e}").into())
    }

    /// Wait for, then return the next Result returned from executors.
    /// Returns `(job_id, executor_id, result)`.
    #[allow(clippy::type_complexity)]
    pub(crate) fn get_next_result(
        &mut self,
        block: bool,
    ) -> Result<(usize, String, Result<(Option<Value>, RunAgain)>)> {
        let flags = if block { WAIT } else { DONTWAIT };

        let msg = self
            .results_socket
            .recv_msg(flags)
            .map_err(|_| "Error receiving result")?;
        let message_string = msg.as_str().ok_or("Could not get message as str")?;
        serde_json::from_str(message_string).map_err(|e| {
            error!("Could not deserialize result from executor (version mismatch?): {e}");
            "Could not Deserialize from zmq message string".into()
        })
    }

    // Send a `Job` for execution to executors.
    //
    // Jobs are routed to two executor pools:
    // - `lib_job_socket`: library functions (lib://), WASM functions (file://,
    //   http://, https://), and sub-flow functions (subflow://) — these run on
    //   the multi-threaded executor pool for parallelism
    // - `general_job_socket`: context functions (context://) — these interact with
    //   the environment and run on a dedicated executor with spawn support
    pub(crate) fn send_job_for_execution(&mut self, payload: &Payload) -> Result<()> {
        let scheme = payload.implementation_url.scheme();
        if matches!(scheme, "lib" | "file" | "http" | "https" | "subflow") {
            self.lib_job_socket
                .send(serde_json::to_string(payload)?.as_bytes(), 0)
                .map_err(|e| format!("Could not send Job for execution: {e}"))?;
        } else {
            self.general_job_socket
                .send(serde_json::to_string(payload)?.as_bytes(), 0)
                .map_err(|e| format!("Could not send context Job for execution: {e}"))?;
        }

        trace!("Job #{}: Payload sent for execution", payload.job_id);

        Ok(())
    }

    /// Send a "DONE"" message to subscribed executors on the `control_socket`
    ///
    /// # Errors
    ///
    /// Returns an error if the message bytes cannot be sent over the control socket
    ///
    pub fn send_done(&mut self) -> Result<()> {
        debug!("Dispatcher announcing DONE");
        self.control_socket
            .send("DONE".as_bytes(), DONTWAIT)
            .chain_err(|| "Could not send 'DONE' message")
    }
}

impl Drop for Dispatcher {
    fn drop(&mut self) {
        if let Err(e) = self.send_done() {
            error!("Error while sending DONE while dropping Dispatcher: {e}");
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {
    use std::time::Duration;

    use serde_json::Value;
    use serial_test::serial;
    use url::Url;

    use flowcore::errors::*;
    use flowcore::RunAgain;
    use flowcore::DONT_RUN_AGAIN;

    use crate::job::Payload;
    use crate::test_helper::fixtures::{get_bind_addresses, get_four_ports};

    /// Create a `Dispatcher` for testing, re-picking ports and retrying a few times to
    /// tolerate the occasional race where another process grabs a freshly picked port.
    fn new_dispatcher() -> (super::Dispatcher, (u16, u16, u16, u16)) {
        for _ in 0..10 {
            let ports = get_four_ports();
            if let Ok(dispatcher) = super::Dispatcher::new(&get_bind_addresses(ports)) {
                return (dispatcher, ports);
            }
        }
        panic!("Could not create dispatcher after 10 attempts");
    }

    #[test]
    #[serial]
    fn test_constructor() {
        let (_dispatcher, _ports) = new_dispatcher();
    }

    #[test]
    #[serial]
    fn set_timeout_to_none() {
        let (mut dispatcher, _ports) = new_dispatcher();
        assert!(dispatcher.set_results_timeout(None).is_ok());
    }

    #[test]
    #[serial]
    fn set_timeout() {
        let (mut dispatcher, _ports) = new_dispatcher();
        assert!(dispatcher
            .set_results_timeout(Some(Duration::from_millis(10)))
            .is_ok());
    }

    /// Helper: connect PULL sockets to both the lib (ports.0) and context
    /// (ports.1) dispatcher queues so tests can observe routing.
    fn connect_both_queues(
        context: &zmq::Context,
        ports: (u16, u16, u16, u16),
    ) -> (zmq::Socket, zmq::Socket) {
        let lib_socket = context
            .socket(zmq::PULL)
            .expect("Could not create lib PULL socket");
        lib_socket
            .connect(&format!("tcp://127.0.0.1:{}", ports.0))
            .expect("Could not connect to lib job socket");
        lib_socket
            .set_rcvtimeo(1000)
            .expect("Could not set timeout");

        let ctx_socket = context
            .socket(zmq::PULL)
            .expect("Could not create context PULL socket");
        ctx_socket
            .connect(&format!("tcp://127.0.0.1:{}", ports.1))
            .expect("Could not connect to context job socket");
        ctx_socket.set_rcvtimeo(100).expect("Could not set timeout");

        (lib_socket, ctx_socket)
    }

    #[test]
    #[serial]
    fn send_lib_job() {
        let payload = Payload {
            job_id: 0,
            input_set: vec![],
            implementation_url: Url::parse("lib://flowstdlib/math/add")
                .expect("Could not parse Url"),
        };

        let (mut dispatcher, ports) = new_dispatcher();
        let context = zmq::Context::new();
        let (lib_socket, ctx_socket) = connect_both_queues(&context, ports);

        assert!(dispatcher.send_job_for_execution(&payload).is_ok());

        let msg = lib_socket
            .recv_msg(0)
            .expect("lib:// job should arrive on lib socket");
        let received: Payload = serde_json::from_str(msg.as_str().unwrap()).unwrap();
        assert_eq!(received.implementation_url.scheme(), "lib");

        assert!(
            ctx_socket.recv_msg(0).is_err(),
            "lib:// job should not be routed to context socket"
        );
    }

    #[test]
    #[serial]
    fn send_context_job() {
        let payload = Payload {
            job_id: 0,
            input_set: vec![],
            implementation_url: Url::parse("context://stdio/stdout").expect("Could not parse Url"),
        };

        let (mut dispatcher, ports) = new_dispatcher();
        let context = zmq::Context::new();
        let (lib_socket, ctx_socket) = connect_both_queues(&context, ports);

        assert!(dispatcher.send_job_for_execution(&payload).is_ok());

        // Context timeout is 100ms; bump it so we can receive
        ctx_socket.set_rcvtimeo(1000).expect("set timeout");
        let msg = ctx_socket
            .recv_msg(0)
            .expect("context:// job should arrive on context socket");
        let received: Payload = serde_json::from_str(msg.as_str().unwrap()).unwrap();
        assert_eq!(received.implementation_url.scheme(), "context");

        assert!(
            lib_socket.recv_msg(0).is_err(),
            "context:// job should not be routed to lib socket"
        );
    }

    #[test]
    #[serial]
    fn send_subflow_job() {
        let payload = Payload {
            job_id: 0,
            input_set: vec![],
            implementation_url: Url::parse("subflow://1").expect("Could not parse Url"),
        };

        let (mut dispatcher, ports) = new_dispatcher();
        let context = zmq::Context::new();
        let (lib_socket, ctx_socket) = connect_both_queues(&context, ports);

        assert!(dispatcher.send_job_for_execution(&payload).is_ok());

        let msg = lib_socket
            .recv_msg(0)
            .expect("subflow:// job should arrive on lib socket");
        let received: Payload = serde_json::from_str(msg.as_str().unwrap()).unwrap();
        assert_eq!(received.implementation_url.scheme(), "subflow");

        assert!(
            ctx_socket.recv_msg(0).is_err(),
            "subflow:// job should not be routed to context socket"
        );
    }

    #[test]
    #[serial]
    fn send_http_wasm_job() {
        let payload = Payload {
            job_id: 0,
            input_set: vec![],
            implementation_url: Url::parse("http://192.168.1.1:12345/path/to/module.wasm")
                .expect("Could not parse Url"),
        };

        let (mut dispatcher, ports) = new_dispatcher();
        let context = zmq::Context::new();
        let (lib_socket, ctx_socket) = connect_both_queues(&context, ports);

        assert!(dispatcher.send_job_for_execution(&payload).is_ok());

        let msg = lib_socket
            .recv_msg(0)
            .expect("http:// WASM job should arrive on lib socket");
        let received: Payload = serde_json::from_str(msg.as_str().unwrap()).unwrap();
        assert_eq!(received.implementation_url.scheme(), "http");

        assert!(
            ctx_socket.recv_msg(0).is_err(),
            "http:// job should not be routed to context socket"
        );
    }

    #[test]
    #[serial]
    fn get_job() {
        let (mut dispatcher, ports) = new_dispatcher();

        let context = zmq::Context::new();
        let results_sink = context
            .socket(zmq::PUSH)
            .expect("Could not create PUSH end of results socket");
        results_sink
            .connect(&format!("tcp://127.0.0.1:{}", ports.2))
            .expect("Could not connect to PULL end of results socket");
        let result: Result<(Option<Value>, RunAgain)> = Ok((None, DONT_RUN_AGAIN));
        results_sink
            .send(
                serde_json::to_string(&(0, "test-executor", result))
                    .expect("Could not convert to serde")
                    .as_bytes(),
                0,
            )
            .expect("Could not send result of Job");

        let received = dispatcher.get_next_result(true);
        assert!(received.is_ok());
        let (job_id, executor_id, _) = received.unwrap();
        assert_eq!(job_id, 0);
        assert_eq!(executor_id, "test-executor");
    }
}

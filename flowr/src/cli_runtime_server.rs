use std::sync::{Arc, Mutex};

use error_chain::bail;
use log::{debug, error, info};

use flowcore::errors::*;
use flowcore::model::metrics::Metrics;
use flowcore::model::submission::Submission;
use flowrlib::run_state::RunState;
use flowrlib::server::Server;

use crate::{ClientMessage, DONT_WAIT, ServerConnection, ServerMessage, WAIT};

/// Get and Send messages to/from the runtime client
pub(crate) struct CliServer {
    pub(crate) runtime_server_connection: Arc<Mutex<ServerConnection>>,
}

impl Server for CliServer {
    // The flow is starting
    fn flow_starting(&mut self) -> Result<()> {
        let _ = self.runtime_server_connection
            .lock()
            .map_err(|_| "Could not lock server connection")?
            .send_and_receive_response::<ServerMessage, ClientMessage>(ServerMessage::FlowStart)?;

        Ok(())
    }

    // See if the runtime client has sent a message to request us to enter the debugger,
    // if so, return Ok(true).
    // A different message or Absence of a message returns Ok(false)
    #[cfg(feature = "debugger")]
    fn should_enter_debugger(&mut self) -> Result<bool> {
        let msg = self
            .runtime_server_connection
            .lock()
            .map_err(|_| "Could not lock server connection")?
            .receive(DONT_WAIT);
        match msg {
            Ok(ClientMessage::EnterDebugger) => {
                debug!("Got EnterDebugger message");
                Ok(true)
            }
            Ok(m) => {
                debug!("Got {:?} message", m);
                Ok(false)
            }
            _ => Ok(false),
        }
    }

    #[cfg(feature = "metrics")]
    fn flow_ended(&mut self, state: &RunState, metrics: Metrics) -> Result<()> {
        self.runtime_server_connection
            .lock()
            .map_err(|_| "Could not lock server connection")?
            .send(ServerMessage::FlowEnd(metrics))?;
        debug!("{}", state);
        Ok(())
    }

    #[cfg(not(feature = "metrics"))]
    fn flow_ended(&mut self) -> flowcore::errors::Result<()> {
        self.runtime_server_connection
            .lock()
            .map_err(|_| "Could not lock server connection")?
            .send(ServerMessage::FlowEnd)?;
        debug!("{}", state);
        Ok(())
    }

    // Loop waiting for one of the following two messages from the client thread:
    //  - `ClientSubmission` with a submission, then return Ok(Some(submission))
    //  - `ClientExiting` then return Ok(None)
    fn wait_for_submission(&mut self) -> Result<Option<Submission>> {
        loop {
            info!("Server is waiting to receive a 'Submission'");
            match self.runtime_server_connection.lock() {
                Ok(guard) => match guard.receive(WAIT) {
                    Ok(ClientMessage::ClientSubmission(submission)) => {
                        debug!(
                            "Server received a submission for execution with manifest_url: '{}'",
                            submission.manifest_url
                        );
                        return Ok(Some(submission));
                    }
                    Ok(ClientMessage::ClientExiting(_)) => return Ok(None),
                    Ok(r) => error!("Server did not expect response from client: '{:?}'", r),
                    Err(e) => bail!("Server error while waiting for submission: '{}'", e),
                },
                _ => {
                    error!("Server could not lock connection");
                    return Ok(None);
                }
            }
        }
    }

    // The flow server is about to exit
    fn server_exiting(&mut self, result: Result<()>) -> Result<()> {
        debug!("Server closing connection");
        let mut connection = self.runtime_server_connection
            .lock()
            .map_err(|e| format!("Could not lock Server Connection: {}", e))?;
        connection.send(ServerMessage::ServerExiting(result))?;
        Ok(())
    }
}
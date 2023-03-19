use std::sync::{Arc, Mutex};

use error_chain::bail;
use log::{debug, error, info, trace};

use flowcore::errors::*;
#[cfg(feature = "metrics")]
use flowcore::model::metrics::Metrics;
use flowcore::model::submission::Submission;
use flowrlib::protocols::SubmissionProtocol;
use flowrlib::run_state::RunState;

#[cfg(feature = "debugger")]
use crate::cli::client_coordinator::{DONT_WAIT, WAIT};
use crate::cli::messages::ClientMessage;
use crate::cli::messages::CoordinatorMessage;
use crate::CoordinatorConnection;

/// Get and Send messages to/from the runtime client
pub(crate) struct CLISubmitter {
    pub(crate) coordinator_connection: Arc<Mutex<CoordinatorConnection>>,
}

impl SubmissionProtocol for CLISubmitter {
    fn flow_execution_starting(&mut self) -> Result<()> {
        let _ = self.coordinator_connection
            .lock()
            .map_err(|_| "Could not lock server connection")?
            .send_and_receive_response::<CoordinatorMessage, ClientMessage>(CoordinatorMessage::FlowStart)?;

        Ok(())
    }

    // See if the runtime client has sent a message to request us to enter the debugger,
    // if so, return Ok(true).
    // A different message or Absence of a message returns Ok(false)
    #[cfg(feature = "debugger")]
    fn should_enter_debugger(&mut self) -> Result<bool> {
        let msg = self
            .coordinator_connection
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
    fn flow_execution_ended(&mut self, state: &RunState, metrics: Metrics) -> Result<()> {
        self.coordinator_connection
            .lock()
            .map_err(|_| "Could not lock server connection")?
            .send(CoordinatorMessage::FlowEnd(metrics))?;
        debug!("{}", state);
        Ok(())
    }

    #[cfg(not(feature = "metrics"))]
    fn flow_execution_ended(&mut self, state: &RunState) -> Result<()> {
        self.coordinator_connection
            .lock()
            .map_err(|_| "Could not lock server connection")?
            .send(CoordinatorMessage::FlowEnd)?;
        debug!("{}", state);
        Ok(())
    }

    // Loop waiting for one of the following two messages from the client thread:
    //  - `ClientSubmission` with a submission, then return Ok(Some(submission))
    //  - `ClientExiting` then return Ok(None)
    fn wait_for_submission(&mut self) -> Result<Option<Submission>> {
        loop {
            info!("Server is waiting to receive a 'Submission'");
            let guard = self.coordinator_connection.lock();
            match guard {
                Ok(locked) =>  {
                    let received = locked.receive(WAIT);
                    match received {
                        Ok(ClientMessage::ClientSubmission(submission)) => {
                            info!("Server received a submission for execution");
                            trace!("\n{}", submission);
                            return Ok(Some(submission));
                        }
                        Ok(ClientMessage::ClientExiting(_)) => return Ok(None),
                        Ok(r) => error!("Server did not expect response from client: '{:?}'", r),
                        Err(e) => bail!("Server error while waiting for submission: '{}'", e),
                    }
                }
                _ => {
                    error!("Server could not lock connection");
                    return Ok(None);
                }
            }
        }
    }

    // The coordinator is about to exit
    fn coordinator_is_exiting(&mut self, result: Result<()>) -> Result<()> {
        debug!("Coordinator exiting");
        let mut connection = self.coordinator_connection
            .lock()
            .map_err(|e| format!("Could not lock Server Connection: {e}"))?;
        connection.send(CoordinatorMessage::CoordinatorExiting(result))
    }
}
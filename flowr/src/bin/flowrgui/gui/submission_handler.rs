use std::sync::{Arc, Mutex};

use error_chain::bail;
use flowcore::errors::*;
use flowcore::model::metrics::Metrics;
use flowcore::model::submission::Submission;
use flowrlib::run_state::RunState;
use flowrlib::submission_handler::SubmissionHandler;
use log::{debug, error, info, trace};

use crate::CoordinatorConnection;
use crate::gui::client_message::ClientMessage;
use crate::gui::coordinator_connection::{DONT_WAIT, WAIT};
use crate::gui::coordinator_message::CoordinatorMessage;

/// A [`SubmissionHandler`] to allow submitting flows for execution from the CLI
pub(crate) struct CLISubmissionHandler {
    coordinator_connection: Arc<Mutex<CoordinatorConnection>>,
}

impl CLISubmissionHandler {
    /// Create a new Submission handler using the connection provided
    pub fn new(connection: Arc<Mutex<CoordinatorConnection>>) -> Self {
        CLISubmissionHandler {
            coordinator_connection: connection,
        }
    }
}

impl SubmissionHandler for CLISubmissionHandler {
    fn flow_execution_starting(&mut self) -> Result<()> {
        self.coordinator_connection
            .lock()
            .map_err(|_| "Could not lock coordinator connection")?
            .send_and_receive_response::<CoordinatorMessage, ClientMessage>(CoordinatorMessage::FlowStart)
            .map(|_| ())
    }

    // See if the runtime client has sent a message to request us to enter the debugger,
    // if so, return Ok(true).
    // A different message or Absence of a message returns Ok(false)
    fn should_enter_debugger(&mut self) -> Result<bool> {
        let msg = self
            .coordinator_connection
            .lock()
            .map_err(|_| "Could not lock coordinator connection")?
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

    fn flow_execution_ended(&mut self, state: &RunState, metrics: Metrics) -> Result<()> {
        self.coordinator_connection
            .lock()
            .map_err(|_| "Could not lock coordinator connection")?
            .send(CoordinatorMessage::FlowEnd(metrics))?;
        debug!("{}", state);
        Ok(())
    }

    // Loop waiting for one of the following two messages from the client thread:
    //  - `ClientSubmission` with a submission, then return Ok(Some(submission))
    //  - `ClientExiting` then return Ok(None)
    fn wait_for_submission(&mut self) -> Result<Option<Submission>> {
        loop {
            info!("Coordinator is waiting to receive a 'Submission'");
            let guard = self.coordinator_connection.lock();
            #[allow(clippy::single_match_else)]
            match guard {
                Ok(locked) =>  {
                    let received = locked.receive(WAIT);
                    match received {
                        Ok(ClientMessage::ClientSubmission(submission)) => {
                            info!("Coordinator received a submission for execution");
                            trace!("\n{}", submission);
                            return Ok(Some(submission));
                        }
                        Ok(ClientMessage::ClientExiting(_)) => return Ok(None),
                        Ok(r) => error!("Coordinator did not expect message from client: '{:?}'", r),
                        Err(e) => bail!("Coordinator error while waiting for submission: '{}'", e),
                    }
                }
                _ => {
                    error!("Coordinator could not lock connection");
                    return Ok(None);
                }
            }
        }
    }

    fn coordinator_is_exiting(&mut self, result: Result<()>) -> Result<()> {
        debug!("Coordinator exiting");
        let mut connection = self.coordinator_connection
            .lock()
            .map_err(|e|
                format!("Could not lock Coordinator Connection: {e}"))?;
        connection.send(CoordinatorMessage::CoordinatorExiting(result))
    }
}
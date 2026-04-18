use std::sync::{Arc, Mutex};

use error_chain::bail;
use flowcore::errors::Result;
use flowcore::model::metrics::Metrics;
use flowcore::model::submission::Submission;
use flowrlib::run_state::RunState;
use flowrlib::submission_handler::SubmissionHandler;
use log::{debug, error, info, trace};

use crate::gui::client_message::ClientMessage;
use crate::gui::coordinator_connection::CoordinatorConnection;
use crate::gui::coordinator_connection::{DONT_WAIT, WAIT};
use crate::gui::coordinator_message::CoordinatorMessage;

/// Submission handler for flowedit
pub(crate) struct CLISubmissionHandler {
    coordinator_connection: Arc<Mutex<CoordinatorConnection>>,
}

impl CLISubmissionHandler {
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
            .send_and_receive_response::<CoordinatorMessage, ClientMessage>(
                CoordinatorMessage::FlowStart,
            )
            .map(|_| ())
    }

    fn should_enter_debugger(&mut self) -> Result<bool> {
        // flowedit does not support debugging — never enter debugger.
        // Must NOT lock the coordinator_connection here, because readline
        // may be holding it while waiting for user input. Locking would
        // block the coordinator's main loop.
        Ok(false)
    }

    fn flow_execution_ended(&mut self, state: &RunState, metrics: Metrics) -> Result<()> {
        self.coordinator_connection
            .lock()
            .map_err(|_| "Could not lock coordinator connection")?
            .send(CoordinatorMessage::FlowEnd(metrics))?;
        debug!("{state}");
        Ok(())
    }

    fn wait_for_submission(&mut self) -> Result<Option<Submission>> {
        loop {
            info!("Coordinator is waiting to receive a 'Submission'");
            let guard = self.coordinator_connection.lock();
            #[allow(clippy::single_match_else)]
            match guard {
                Ok(locked) => {
                    let received = locked.receive(WAIT);
                    match received {
                        Ok(ClientMessage::ClientSubmission(submission)) => {
                            info!("Coordinator received a submission for execution");
                            trace!("\n{submission}");
                            return Ok(Some(submission));
                        }
                        Ok(ClientMessage::ClientExiting(_)) => return Ok(None),
                        Ok(r) => error!("Coordinator did not expect message from client: '{r:?}'"),
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
        let mut connection = self
            .coordinator_connection
            .lock()
            .map_err(|e| format!("Could not lock Coordinator Connection: {e}"))?;
        connection.send(CoordinatorMessage::CoordinatorExiting(result))
    }
}

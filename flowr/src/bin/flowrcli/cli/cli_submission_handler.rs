use std::sync::mpsc;

use log::{debug, info, trace};

use flowcore::errors::Result;
#[cfg(feature = "metrics")]
use flowcore::model::metrics::Metrics;
use flowcore::model::submission::Submission;
use flowrlib::client_protocol::{BoundaryOutputEntry, CoordinatorMessage};
use flowrlib::run_state::RunState;
use flowrlib::submission_handler::SubmissionHandler;

use crate::context::ContextIO;

/// A [`SubmissionHandler`] for the CLI runner.
///
/// Uses channel-based `ContextIO` to communicate with the bridge thread that
/// owns the ZMQ `CoordinatorConnection`. No mutex needed.
pub(crate) struct CLISubmissionHandler {
    context_io: ContextIO,
    submission_rx: mpsc::Receiver<Submission>,
}

impl CLISubmissionHandler {
    pub fn new(context_io: ContextIO, submission_rx: mpsc::Receiver<Submission>) -> Self {
        CLISubmissionHandler {
            context_io,
            submission_rx,
        }
    }
}

impl SubmissionHandler for CLISubmissionHandler {
    fn flow_execution_starting(&mut self) -> Result<()> {
        let _ = self
            .context_io
            .send_and_receive(CoordinatorMessage::FlowStart)?;
        Ok(())
    }

    #[cfg(feature = "debugger")]
    fn should_enter_debugger(&mut self) -> Result<bool> {
        Ok(false)
    }

    #[cfg(feature = "metrics")]
    fn flow_execution_ended(&mut self, state: &RunState, metrics: Metrics) -> Result<()> {
        let boundary_outputs: Vec<BoundaryOutputEntry> = state
            .boundary_outputs()
            .iter()
            .map(|bo| BoundaryOutputEntry {
                connection: bo.connection.clone(),
                value: bo.value.clone(),
            })
            .collect();
        self.context_io
            .send_and_receive(CoordinatorMessage::FlowEnd(boundary_outputs, metrics))?;
        debug!("{state}");
        Ok(())
    }

    #[cfg(not(feature = "metrics"))]
    fn flow_execution_ended(&mut self, state: &RunState) -> Result<()> {
        let boundary_outputs: Vec<BoundaryOutputEntry> = state
            .boundary_outputs()
            .iter()
            .map(|bo| BoundaryOutputEntry {
                connection: bo.connection.clone(),
                value: bo.value.clone(),
            })
            .collect();
        self.context_io
            .send_and_receive(CoordinatorMessage::FlowEnd(boundary_outputs))?;
        debug!("{}", state);
        Ok(())
    }

    fn wait_for_submission(&mut self) -> Result<Option<Submission>> {
        info!("Coordinator is waiting to receive a 'Submission'");
        // Tell the bridge thread to switch to ZMQ receive mode for the next submission
        self.context_io
            .send_and_receive(CoordinatorMessage::Invalid)?;
        match self.submission_rx.recv() {
            Ok(submission) => {
                info!("Coordinator received a submission for execution");
                trace!("\n{submission}");
                Ok(Some(submission))
            }
            Err(_) => Ok(None),
        }
    }

    fn coordinator_is_exiting(&mut self, result: Result<()>) -> Result<()> {
        debug!("Coordinator exiting");
        self.context_io
            .send_and_receive(CoordinatorMessage::CoordinatorExiting(result))
            .map(|_| ())
    }
}

use std::sync::mpsc;

use flowcore::errors::Result;
use flowcore::model::metrics::Metrics;
use flowcore::model::submission::Submission;
use flowrlib::run_state::RunState;
use flowrlib::submission_handler::SubmissionHandler;
use log::{debug, info, trace};

use crate::connection_manager;
use crate::context::ContextIO;
use crate::gui::coordinator_message::CoordinatorMessage;

/// A [`SubmissionHandler`] for the GUI runner.
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

    fn should_stop(&mut self) -> Result<bool> {
        if connection_manager::take_stop_request() {
            debug!("Stop requested by user");
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn jobs_created(&mut self, count: usize) {
        connection_manager::set_job_count(count);
    }

    fn flow_execution_ended(
        &mut self,
        state: &RunState,
        #[cfg(feature = "metrics")] metrics: Metrics,
    ) -> Result<()> {
        #[cfg(feature = "metrics")]
        self.context_io
            .send_and_receive(CoordinatorMessage::FlowEnd(metrics))?;
        debug!("{state}");
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

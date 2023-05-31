use flowcore::errors::*;
#[cfg(feature = "metrics")]
use flowcore::model::metrics::Metrics;
use flowcore::model::submission::Submission;

use crate::run_state::RunState;

/// Programs that wish to submit a flow for execution via a
/// [Submission][flowcore::model::submission::Submission] and
/// then track it's execution (such as a CLI or a UI) should implement this trait
pub trait SubmissionHandler {
    /// Execution of the flow is starting
    fn flow_execution_starting(&mut self) -> Result<()>;

    /// The [Coordinator][crate::coordinator::Coordinator] executing the flow periodically
    /// will check if there has been a request to enter the debugger.
    #[cfg(feature = "debugger")]
    fn should_enter_debugger(&mut self) -> Result<bool>;

    /// The [Coordinator][crate::coordinator::Coordinator] informs the submitter that the execution
    /// of the flow has ended
    fn flow_execution_ended(&mut self, state: &RunState,
                            #[cfg(feature = "metrics")] metrics: Metrics
    ) -> Result<()>;

    /// The [Coordinator][crate::coordinator::Coordinator] wait for a
    /// [Submission][flowcore::model::submission::Submission] to be sent for execution
    fn wait_for_submission(&mut self) -> Result<Option<Submission>>;

    /// The [Coordinator][crate::coordinator::Coordinator] is about to exit
    fn coordinator_is_exiting(&mut self, result: Result<()>) -> Result<()>;
}
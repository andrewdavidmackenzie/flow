use flowcore::errors::*;
use flowcore::model::submission::Submission;
use flowrlib::server::Server;

pub(crate) struct NullServer;

#[allow(unused_variables)]
impl Server for NullServer {
    fn flow_starting(&mut self) -> Result<()> {
        Ok(())
    }

    #[cfg(feature = "debugger")]
    fn should_enter_debugger(&mut self) -> Result<bool> {
        Ok(false)
    }

    #[cfg(feature = "metrics")]
    fn flow_ended(&mut self, state: &RunState, metrics: Metrics) -> Result<()> {
        Ok(())
    }

    #[cfg(not(feature = "metrics"))]
    fn flow_ended(&mut self) -> flowcore::errors::Result<()> {
        Ok(())
    }

    fn wait_for_submission(&mut self) -> Result<Option<Submission>> {
        Ok(None)
    }

    fn server_exiting(&mut self, result: Result<()>) -> Result<()> {
       Ok(())
    }
}
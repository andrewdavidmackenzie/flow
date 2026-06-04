#[allow(clippy::module_name_repetitions)]
pub mod cli_client;
#[cfg(feature = "debugger")]
#[allow(clippy::module_name_repetitions)]
pub mod cli_debug_handler;
#[allow(clippy::module_name_repetitions)]
pub mod cli_submission_handler;
pub mod connections;
pub mod coordinator_message;
#[cfg(feature = "debugger")]
pub mod debug_message;
pub(crate) mod test_helper;

#[allow(clippy::module_name_repetitions)]
pub mod cli_client;
pub(crate) mod test_helper;
/// 'debug' defines structs passed between the Server and the Client regarding debug events
/// and client responses to them
#[cfg(feature = "debugger")]
pub mod debug_message;
#[cfg(feature = "debugger")]
#[allow(clippy::module_name_repetitions)]
pub mod cli_debug_client;
#[cfg(feature = "debugger")]
#[allow(clippy::module_name_repetitions)]
pub mod cli_debug_handler;
#[allow(clippy::module_name_repetitions)]
pub mod cli_submission_handler;
pub mod connections;
pub mod coordinator_message;

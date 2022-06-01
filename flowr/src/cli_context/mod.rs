/// Module of context functions for Cli Flowr Runner

mod args;
mod file;
mod image;
mod stdio;
// Test helper functions
pub(crate) mod test_helper;
/// 'debug' defines structs passed between the Server and the Client regarding debug events
/// and client responses to them
#[cfg(feature = "debugger")]
pub mod debug_server_message;
#[cfg(feature = "debugger")]
pub mod cli_debug_client;
pub mod cli_runtime_client;
#[cfg(feature = "debugger")]
pub mod cli_debug_server;
pub mod cli_server;




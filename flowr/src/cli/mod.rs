/// Module of context functions for Cli Flowr Runner

#[cfg(feature = "context")]
mod args;
#[cfg(feature = "context")]
mod file;
#[cfg(feature = "context")]
mod image;
#[cfg(feature = "context")]
mod stdio;

pub mod cli_runtime_client;

#[cfg(feature = "context")]
pub(crate) mod test_helper;
/// 'debug' defines structs passed between the Server and the Client regarding debug events
/// and client responses to them
#[cfg(feature = "debugger")]
pub mod debug_server_message;
#[cfg(feature = "debugger")]
pub mod cli_debug_client;
#[cfg(feature = "debugger")]
pub mod cli_debug_server;
#[cfg(feature = "context")]
pub mod cli_server;
#[cfg(feature = "submission")]
pub mod cli_submitter;
#[cfg(any(feature = "submission", feature = "context"))]
/// message_queue implementation of the communications between the runtime client, debug client and
/// the runtime server and debug server.
pub mod client_server;
/// runtime_messages is the enum for the different messages sent back and fore between the client
/// and server implementation of the CLI context functions
pub mod runtime_messages;




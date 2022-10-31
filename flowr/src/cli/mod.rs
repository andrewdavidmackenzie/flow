/// Module of context functions for Cli Flowr Runner

mod args;
mod file;
mod image;
mod stdio;

pub mod cli_runtime_client;

pub(crate) mod test_helper;
/// 'debug' defines structs passed between the Server and the Client regarding debug events
/// and client responses to them
#[cfg(feature = "debugger")]
pub mod debug_server_message;
#[cfg(feature = "debugger")]
pub mod cli_debug_client;
#[cfg(feature = "debugger")]
pub mod cli_debug_server;
pub mod cli_context;
pub mod cli_submitter;
/// message_queue implementation of the communications between the runtime client, debug client and
/// the runtime server and debug server.
pub mod client_server;
/// runtime_messages is the enum for the different messages sent back and fore between the client
/// and server implementation of the CLI context functions
pub mod runtime_messages;




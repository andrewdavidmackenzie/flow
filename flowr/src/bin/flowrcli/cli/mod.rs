pub mod cli_client;
pub(crate) mod test_helper;
/// 'debug' defines structs passed between the Server and the Client regarding debug events
/// and client responses to them
#[cfg(feature = "debugger")]
pub mod debug_message;
#[cfg(feature = "debugger")]
pub mod cli_debug_client;
#[cfg(feature = "debugger")]
pub mod cli_debug_handler;
pub mod cli_submission_handler;
/// message_queue implementation of the communications between the runtime client, debug client and
/// the runtime server and debug server.
pub mod connections;
/// runtime_messages is the enum for the different messages sent back and fore between the client
/// and server implementation of the CLI context functions
pub mod coordinator_message;

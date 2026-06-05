#![deny(clippy::unwrap_used, clippy::expect_used)]
//! `flowdblib` is the debugger client library for flow programs. It provides shared types
//! and connection logic used by the `flowdb` CLI debugger and can be reused by other
//! debugger frontends such as a GUI debugger.

/// ZMQ client connection (REQ socket) used by debug clients to connect to the debug server
pub mod client_connection;

/// ZMQ server connection (REP socket) used by the debug server in the coordinator
pub mod coordinator_connection;

/// [`DebugClient`][debug_client::DebugClient] — a CLI REPL debug client
pub mod debug_client;

/// [`DebugHandler`][debug_handler::DebugHandler] implements the
/// [`DebuggerHandler`][flowrlib::debugger_handler::DebuggerHandler] trait,
/// bridging the coordinator to a debug client over ZMQ
pub mod debug_handler;

/// [`DebugServerMessage`][debug_server_message::DebugServerMessage] — messages sent from the
/// debug server to debug clients
pub mod debug_server_message;

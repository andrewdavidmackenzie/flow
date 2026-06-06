#![deny(clippy::unwrap_used, clippy::expect_used)]
//! `flowdblib` is the debugger client library for flow programs. It provides the
//! [`DebugClient`][debug_client::DebugClient] REPL used by the `flowdb` binary.
//!
//! Shared debug types (connections, messages, handler) are in `flowrlib`.

/// [`DebugClient`][debug_client::DebugClient] — a CLI REPL debug client
pub mod debug_client;

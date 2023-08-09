#![deny(missing_docs)]
#![warn(clippy::unwrap_used)]
//! `flowstdlib` is a library of flows and functions that can be used from flows.
//!
//! The flow and function definition are used at compile time when compile flows that reference
//! it.
//!
//! At run-time, library entries can be of two types, indicated by their
//! [ImplementationLocator][flowcore::model::lib_manifest::ImplementationLocator]
//! - [Native][flowcore::model::lib_manifest::ImplementationLocator::Native] - a native binary
//! (containing) all functions is built and linked by a flow
//! runner program (e.g. `flowr`) so that any function referenced by a flow is executed natively
//! at native speeds. `flowr` offers the user control using this via the `-n, --native`
//! command line option.
//! - [RelativePath][flowcore::model::lib_manifest::ImplementationLocator::RelativePath] - functions
//! are compiled to WASM files and located within the library at runtime by the flow runner using
//! this file path relative to the lib root. If either the library if not linked natively, or the
//! `-n, --native` command line option is not used, when the function is referenced by a flow being
//! run, it is loaded and executed in a WASM runtime.

/// Functions and flows to control the flow of data in a flow based on control inputs.
pub mod control;

/// Some generic processes that act on data.
pub mod data;

/// Math Functions and flows
pub mod math;

/// Functions for the formatting of values and conversion from one type to another.
pub mod fmt;

/// Use [manifest::get_manifest] to get the natively/statically linked
/// [LibraryManifest][flowcore::model::lib_manifest::LibraryManifest] for this library
/// to get access to everything `error_chain` creates.
pub mod manifest;

/// provides [Error][errors::Error] that other modules in this crate will `use errors::*;`
pub mod errors;

#[cfg(test)]
pub mod test {
    use std::env;
    use std::io::Read;
    use std::path::PathBuf;
    use std::process::{Command, Stdio};

    fn get_context_root() -> PathBuf {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let samples_dir = manifest_dir.parent().ok_or("Could not get parent dir")
            .expect("Could not get parent dir");
        samples_dir.join("flowr/src/bin/flowrcli/context")
    }

    pub fn execute_flow(filepath: PathBuf) -> String {
        let mut command = Command::new("flowc");
        let context_root = get_context_root();
        let command_args = vec![
            "-C", context_root.to_str().expect("Could not get context root"),
            filepath.to_str().expect("Couldn't convert file path to string")];

        // spawn the 'flowc' child process
        let mut runner = command
            .args(command_args)
            .stdin(Stdio::inherit())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("Couldn't spawn flowc to run test flow");

        let result = runner.wait().expect("failed to wait on child");

        // read it's stdout
        let mut output = String::new();
        if let Some(ref mut stdout) = runner.stdout {
            stdout.read_to_string(&mut output).expect("Could not read stdout");
        }

        assert!(result.success(), );
        output
    }

}
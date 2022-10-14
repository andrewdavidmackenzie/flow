#![deny(missing_docs)]
#![warn(clippy::unwrap_used)]
//! `flowrex` is the minimal flow executor for remote nodes.
/// It attempts to be as small as possible, and only accepts jobs for execution over the network
/// and does not load flows, accept flow submissions run a coordinator or access the file system.
/// Any implementations are either preloaded static linked binary functions or loaded from WASM
/// from peers.

use std::{env, thread};
use std::path::PathBuf;
use std::process::exit;
use std::sync::Arc;

use clap::{Arg, ArgMatches, Command};
use log::{error, info, warn};
use simpath::Simpath;
use simplog::SimpleLogger;
#[cfg(feature = "flowstdlib")]
use url::Url;

use flowcore::errors::*;
use flowcore::meta_provider::MetaProvider;
use flowcore::provider::Provider;
use flowrlib::executor::Executor;
use flowrlib::info as flowrlib_info;

/// We'll put our errors in an `errors` module, and other modules in this crate will
/// `use crate::errors::*;` to get access to everything `error_chain` creates.
pub mod errors;

/// Main for flowr binary - call `run()` and print any error that results or exit silently if OK
fn main() {
    match run() {
        Err(ref e) => {
            eprintln!("{}", e);
            for e in e.iter().skip(1) {
                eprintln!("caused by: {}", e);
            }
            exit(1);
        }
        Ok(_) => exit(0),
    }
}

fn run() -> Result<()> {
    info!(
        "'{}' version {}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    );
    info!("'flowrlib' version {}", flowrlib_info::version());

    let matches = get_matches();

    SimpleLogger::init_prefix_timestamp(
        matches.get_one::<String>("verbosity").map(|s| s.as_str()),
        true, false);

    let num_threads = num_threads(&matches);

    server(num_threads)?;

    info!("'{}' has exited", env!("CARGO_PKG_NAME"));

    Ok(())
}

// Create a new `Coordinator`, pre-load any libraries in native format that we want to have before
// loading a flow and it's library references, then enter the `submission_loop()` accepting and
// executing flows submitted for execution, executing each one using the `Coordinator`
fn server(num_threads: usize) -> Result<()> {
    let provider = Arc::new(MetaProvider::new(Simpath::new(""),
        PathBuf::from("/"))) as Arc<dyn Provider>;
    #[allow(unused_mut)]
    let mut executor = Executor::new(provider, num_threads, None);

    #[cfg(feature = "flowstdlib")]
    executor.add_lib(
        flowstdlib::manifest::get_manifest()
            .chain_err(|| "Could not get 'native' flowstdlib manifest")?,
        Url::parse("memory://")? // Statically linked library has no resolved Url
    )?;

    Ok(())
}

// Determine the number of threads to use to execute flows, with a default of the number of cores
// in the device, or any override from the command line.
fn num_threads(matches: &ArgMatches) -> usize {
    let mut num_threads: usize = 0;

    if let Some(threads) = matches.get_one::<usize>("threads") {
        if threads < &1 {
            error!("Minimum number of additional threads is '1', \
            so option has been overridden to be '1'");
            num_threads = 1;
        } else {
            num_threads = *threads;
        }
    }

    if num_threads == 0 {
        num_threads = thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
    }

    num_threads
}

// Parse the command line arguments using clap
fn get_matches() -> ArgMatches {
    let app = Command::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"));

    let app = app
        .arg(Arg::new("threads")
            .short('t')
            .long("threads")
            .number_of_values(1)
            .value_name("THREADS")
            .help("Set number of threads to use to execute jobs (min: 1, default: cores available)"))
        .arg(Arg::new("verbosity")
            .short('v')
            .long("verbosity")
            .number_of_values(1)
            .value_name("VERBOSITY_LEVEL")
            .help("Set verbosity level for output (trace, debug, info, warn, error (default))"));

    app.get_matches()
}
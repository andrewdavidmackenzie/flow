#![deny(missing_docs)]
#![warn(clippy::unwrap_used)]
//! `flowrex` is the minimal flow executor for remote nodes.
/// It attempts to be as small as possible, and only accepts jobs for execution over the network
/// and does not load flows, accept flow submissions run a coordinator or access the file system.
/// Any implementations are either preloaded static linked binary functions or loaded from WASM
/// from peers.

use std::{env, thread};
use std::process::exit;
use std::sync::Arc;

use clap::{App, AppSettings, Arg, ArgMatches};
use log::{error, info, warn};
use simplog::SimpleLogger;
#[cfg(feature = "flowstdlib")]
use url::Url;

use flowcore::errors::*;
use flowcore::p2p_provider::P2pProvider;
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

    SimpleLogger::init_prefix_timestamp(matches.value_of("verbosity"), true, false);

    let num_threads = num_threads(&matches);


    let provider = Arc::new(P2pProvider::new()) as Arc<dyn Provider>;
    #[allow(unused_mut)]
    let mut executor = Executor::new();

    #[cfg(feature = "flowstdlib")]
    executor.add_lib(
        flowstdlib::manifest::get_manifest()
            .chain_err(|| "Could not get 'native' flowstdlib manifest")?,
        Url::parse("memory://")? // Statically linked library has no resolved Url
    )?;

    executor.start(provider, num_threads)?;

    thread::park();

    info!("'{}' has exited", env!("CARGO_PKG_NAME"));

    Ok(())
}

// Determine the number of threads to use to execute flows, with a default of the number of cores
// in the device, or any override from the command line.
fn num_threads(matches: &ArgMatches) -> usize {
    let mut num_threads: usize = 0;

    if let Some(value) = matches.value_of("threads") {
        if let Ok(threads) = value.parse::<usize>() {
            if threads < 1 {
                error!("Minimum number of additional threads is '1', \
                so option has been overridden to be '1'");
                num_threads = 1;
            } else {
                num_threads = threads;
            }
        }
    }

    if num_threads == 0 {
        num_threads = thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
    }

    num_threads
}

// Parse the command line arguments using clap
fn get_matches() -> ArgMatches {
    let app = App::new(env!("CARGO_PKG_NAME"))
        .setting(AppSettings::TrailingVarArg)
        .version(env!("CARGO_PKG_VERSION"));

    let app = app
        .arg(Arg::with_name("threads")
            .short('t')
            .long("threads")
            .takes_value(true)
            .value_name("THREADS")
            .help("Set number of threads to use to execute jobs (min: 1, default: cores available)"))
        .arg(Arg::with_name("verbosity")
            .short('v')
            .long("verbosity")
            .takes_value(true)
            .value_name("VERBOSITY_LEVEL")
            .help("Set verbosity level for output (trace, debug, info, warn, error (default))"))
        .arg(
        Arg::with_name("native")
            .short('n')
            .long("native")
            .help("Link with native (not WASM) version of flowstdlib"),
    );

    app.get_matches()
}
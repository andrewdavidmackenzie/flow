//! `flowrex` is the minimal executor of flow jobs. It loads a native version of 'flowstdlib'
//! flow library to allow execution of jobs using functions provided by 'flowstdlib', but it does
//! *not* load 'context' and hence will not execute any jobs interacting with the context.
//! It attempts to be as small as possible, and only accepts jobs for execution over the network
//! and does not load flows, accept flow submissions run a coordinator or access the file system.
//! Any implementations are either preloaded static linked binary functions or loaded from WASM
//! from peers.

use core::str::FromStr;
use std::{env, thread};
use std::path::PathBuf;
use std::process::exit;
use std::sync::Arc;

use clap::{Arg, ArgMatches, Command};
use env_logger::Builder;
use log::{error, info, LevelFilter, trace};
use simpath::Simpath;
use simpdiscoverylib::BeaconListener;
#[cfg(feature = "flowstdlib")]
use url::Url;

use flowcore::errors::{Result, ResultExt};
use flowcore::meta_provider::MetaProvider;
use flowcore::provider::Provider;
use flowrlib::executor::Executor;
use flowrlib::info as flowrlib_info;
use flowrlib::services::{CONTROL_SERVICE_NAME, JOB_QUEUES_DISCOVERY_PORT,
                         JOB_SERVICE_NAME, RESULTS_JOB_SERVICE_NAME};

/// We'll put our errors in an `errors` module, and other modules in this crate will
/// `use crate::errors::*;` to get access to everything `error_chain` creates.
pub mod errors;

/// Try to discover a server offering a particular service by name
fn discover_service(discovery_port: u16, name: &str) -> Result<String> {
    let listener = BeaconListener::new(name.as_bytes(), discovery_port)?;
    let beacon = listener.wait(None)?;
    let server_address = format!("{}:{}", beacon.service_ip, beacon.service_port);
    Ok(server_address)
}

/// Main for flowrex binary - call `run()` and print any error that results or exit silently if OK
fn main() {
    match run() {
        Err(ref e) => {
            error!("{e}");
            for e in e.iter().skip(1) {
                error!("caused by: {e}");
            }

            // The backtrace is generated if env var `RUST_BACKTRACE` is set to `1` or `full`
            if let Some(backtrace) = e.backtrace() {
                error!("backtrace: {backtrace:?}");
            }

            exit(1);
        }
        Ok(()) => exit(0),
    }
}

fn run() -> Result<()> {
    let matches = get_matches();

    let default = String::from("error");
    let verbosity = matches.get_one::<String>("verbosity").unwrap_or(&default);
    let level = LevelFilter::from_str(verbosity).unwrap_or(LevelFilter::Error);
    let mut builder = Builder::from_default_env();
    builder.filter_level(level).init();

    info!(
        "'{}' version {}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    );
    info!("'flowrlib' version {}", flowrlib_info::version());

    start_executors(num_threads(&matches))?;

    info!("'{}' has exited", env!("CARGO_PKG_NAME"));

    Ok(())
}

fn start_executors(num_threads: usize) -> Result<()> {
    // loop, re-discovering flowr announced services that change network address on each run
    loop {
        #[allow(unused_mut)]
        let mut executor = Executor::new();

        #[cfg(feature = "flowstdlib")]
        executor.add_lib(
            flowstdlib::manifest::get()
                .chain_err(|| "Could not get 'native' flowstdlib manifest")?,
            Url::parse("memory://")?
        )?;
        trace!("'flowstdlib' loaded into '{}' executors", env!("CARGO_PKG_NAME"));

        let provider = Arc::new(MetaProvider::new(Simpath::new(""),
            PathBuf::from("/"))) as Arc<dyn Provider>;

        let job_service = format!("tcp://{}",
                                  discover_service(JOB_QUEUES_DISCOVERY_PORT,
                                                   JOB_SERVICE_NAME)?);
        let results_service = format!("tcp://{}",
                                      discover_service(JOB_QUEUES_DISCOVERY_PORT,
                                                       RESULTS_JOB_SERVICE_NAME)?);

        let control_service = format!("tcp://{}",
                                      discover_service(JOB_QUEUES_DISCOVERY_PORT,
                                                       CONTROL_SERVICE_NAME)?);

        trace!("Starting '{}' executors", env!("CARGO_PKG_NAME"));
        executor.start(&provider, num_threads, &job_service, &results_service,
                       &control_service);

        trace!("Waiting for all executors to complete");
        executor.wait();
        trace!("All executors completed, exiting");
    }
}

// Determine the number of threads to use to execute flows
// - default (if value is not provided on the command line) to the "available_parallelism()"
#[allow(clippy::redundant_closure_for_method_calls)]
fn num_threads(matches: &ArgMatches) -> usize {
    match matches.get_one::<usize>("threads") {
        Some(num_threads) => *num_threads,
        None => thread::available_parallelism().map(|n| n.get()).unwrap_or(1)
    }
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
            .value_parser(clap::value_parser!(usize))
            .value_name("THREADS")
            .help("Set number of threads to use to execute jobs (default: cores available)"))
        .arg(Arg::new("verbosity")
            .short('v')
            .long("verbosity")
            .number_of_values(1)
            .value_name("VERBOSITY_LEVEL")
            .help("Set verbosity level for output (trace, debug, info, warn, error (default), off)"));

    app.get_matches()
}
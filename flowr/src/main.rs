// TODO #![deny(missing_docs)]
//! `flowr` is the flow runner. It reads a `flow` `Manifest` produced
//! by a flow compiler, such as `flowc`` that describes the network of collaborating functions
//! that are executed to execute the flow.
//!
//! Use `flowr` or `flowr --help` or `flowr -h` at the comment line to see the command line options

#[macro_use]
extern crate error_chain;

use std::env;
use std::process::exit;

use clap::{App, AppSettings, Arg, ArgMatches};
use log::{error, info};
use simplog::simplog::SimpleLogger;
use url::Url;

use cli_debug_client::CLIDebugClient;
use flowrlib::coordinator::{Coordinator, Submission};
use flowrlib::info as flowrlib_info;
use flowrlib::runtime::Response::ClientSubmission;
use provider::args::url_from_string;

use crate::cli_runtime_client::CLIRuntimeClient;

mod cli_debug_client;
mod cli_runtime_client;

// We'll put our errors in an `errors` module, and other modules in this crate will
// `use crate::errors::*;` to get access to everything `error_chain!` creates.
#[doc(hidden)]
pub mod errors {
    // Create the Error, ErrorKind, ResultExt, and Result types
    error_chain! {}
}

#[doc(hidden)]
error_chain! {
    types {
        Error, ErrorKind, ResultExt, Result;
    }

    foreign_links {
        Provider(provider::errors::Error);
        Runtime(flowrlib::errors::Error);
        Io(std::io::Error);
    }
}

fn main() {
    match run() {
        Err(ref e) => {
            println!("error: {}", e);

            for e in e.iter().skip(1) {
                println!("caused by: {}", e);
            }

            // The backtrace is not always generated. Try to run this example
            // with `RUST_BACKTRACE=1`.
            if let Some(backtrace) = e.backtrace() {
                println!("backtrace: {:?}", backtrace);
            }

            exit(1);
        }
        Ok(_) => {
            exit(0);
        }
    }
}

fn run() -> Result<()> {
    let matches = get_matches();

    SimpleLogger::init(matches.value_of("verbosity"));
    let debugger = matches.is_present("debugger");
    let native = matches.is_present("native");

    let (runtime_connection, debugger_connection) = Coordinator::connect(num_threads(&matches, debugger), native);

    if cfg!(feature = "single_process") || matches.is_present("client") {
        let flow_manifest_url = parse_flow_url(&matches)?;
        let flow_args = get_flow_args(&matches, &flow_manifest_url);
        let submission = Submission::new(&flow_manifest_url.to_string(),
                                         num_parallel_jobs(&matches, debugger),
                                         debugger);

        runtime_connection.client_send(ClientSubmission(submission))?;

        CLIDebugClient::start(debugger_connection);
        CLIRuntimeClient::start(runtime_connection,
                                flow_args,
                                #[cfg(feature = "metrics")]
                                    matches.is_present("metrics"),
        );
    }

    Ok(())
}


/*
    Determine the number of threads to use to execute flows, with a default of the number of cores
    in the device, or any override from the command line.

    If debugger=true, then default to 0 threads, unless overridden by an argument
*/
fn num_threads(matches: &ArgMatches, debugger: bool) -> usize {
    if debugger {
        info!("Due to debugger option being set, number of threads has been forced to 1");
        return 1;
    }

    match matches.value_of("threads") {
        Some(value) => {
            match value.parse::<i32>() {
                Ok(mut threads) => {
                    if threads < 1 {
                        error!("Minimum number of additional threads is '1', so option has been overridden to be '1'");
                        threads = 1;
                    }
                    threads as usize
                }
                Err(_) => {
                    error!("Error parsing the value for number of threads '{}'", value);
                    num_cpus::get()
                }
            }
        }
        None => num_cpus::get()
    }
}

/*
    Determine the number of parallel jobs to be run in parallel based on a default of 2 times
    the number of cores in the device, or any override from the command line.
*/
fn num_parallel_jobs(matches: &ArgMatches, debugger: bool) -> usize {
    match matches.value_of("jobs") {
        Some(value) => {
            match value.parse::<i32>() {
                Ok(mut jobs) => {
                    if jobs < 1 {
                        error!("Minimum number of parallel jobs is '0', so option of '{}' has been overridden to be '1'",
                               jobs);
                        jobs = 1;
                    }
                    jobs as usize
                }
                Err(_) => {
                    error!("Error parsing the value for number of parallel jobs '{}'", value);
                    2 * num_cpus::get()
                }
            }
        }
        None => {
            if debugger {
                info!("Due to debugger option being set, max number of parallel jobs has defaulted to 1");
                1
            } else {
                2 * num_cpus::get()
            }
        }
    }
}

/*
    Parse the command line arguments using clap
*/
fn get_matches<'a>() -> ArgMatches<'a> {
    let app = App::new(env!("CARGO_PKG_NAME"))
        .setting(AppSettings::TrailingVarArg)
        .version(env!("CARGO_PKG_VERSION"));

    let app = app.arg(Arg::with_name("jobs")
        .short("j")
        .long("jobs")
        .takes_value(true)
        .value_name("MAX_JOBS")
        .help("Set maximum number of jobs that can be running in parallel)"))
        .arg(Arg::with_name("threads")
            .short("t")
            .long("threads")
            .takes_value(true)
            .value_name("THREADS")
            .help("Set number of threads to use to execute jobs (min: 1, default: cores available)"))
        .arg(Arg::with_name("verbosity")
            .short("v")
            .long("verbosity")
            .takes_value(true)
            .value_name("VERBOSITY_LEVEL")
            .help("Set verbosity level for output (trace, debug, info, warn, error (default))"))
        .arg(Arg::with_name("flow-manifest")
            .help("the file path of the 'flow' manifest file")
            .required(true)
            .index(1))
        .arg(Arg::with_name("flow-arguments")
            .multiple(true)
            .help("A list of arguments to pass to the flow when executed."));

    #[cfg(feature = "distributed")]
        let app = app.arg(Arg::with_name("server")
        .short("s")
        .long("server")
        .help("Launch as flowr server"));

    #[cfg(feature = "distributed")]
        let app = app.arg(Arg::with_name("client")
        .short("c")
        .long("client")
        .help("Launch as flowr client"));

    #[cfg(feature = "debugger")]
        let app = app.arg(Arg::with_name("debugger")
        .short("d")
        .long("debugger")
        .help("Enable the debugger when running a flow"));

    #[cfg(feature = "metrics")]
        let app = app.arg(Arg::with_name("metrics")
        .short("m")
        .long("metrics")
        .help("Calculate metrics during flow execution and print them out when done"));

    #[cfg(feature = "native")]
        let app = app.arg(Arg::with_name("native")
        .short("n")
        .long("native")
        .help("Use native libraries when possible"));

    app.get_matches()
}

/*
    Parse the command line arguments passed onto the flow itself
*/
fn parse_flow_url(matches: &ArgMatches) -> Result<Url> {
    info!("'{}' version {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
    info!("'flowrlib' version {}", flowrlib_info::version());

    let cwd = env::current_dir().chain_err(|| "Could not get current working directory value")?;
    let cwd_url = Url::from_directory_path(cwd)
        .map_err(|_| "Could not form a Url for the current working directory")?;

    url_from_string(&cwd_url, matches.value_of("flow-manifest"))
        .chain_err(|| "Unable to parse the URL of the manifest of the flow to run")
}

/*
    Set environment variable with the args this will not be unique, but it will be used very
    soon and removed
*/
fn get_flow_args(matches: &ArgMatches, flow_manifest_url: &Url) -> Vec<String> {
    // arg #0 is the flow url
    let mut flow_args: Vec<String> = vec!(flow_manifest_url.to_string());

    // append any other arguments for the flow passed from the command line
    if let Some(args) = matches.values_of("flow-arguments") {
        flow_args.extend(args.map(|arg| arg.to_string()));
    }

    flow_args
}
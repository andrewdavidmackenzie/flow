// TODO #![deny(missing_docs)]
//! `flowr` is the flow runtime or flow runner. It reads a `flow` `Manifest` produced
//! by a flow compiler, such as `flowc`` that describes the network of collaborating functions
//! that are executed to execute the flow.
//!
//! Use `flowr` or `flowr --help` or `flowr -h` at the comment line to see the command line options
//!
extern crate clap;
#[macro_use]
extern crate error_chain;
extern crate flow_impl;
extern crate flowrlib;
#[macro_use]
extern crate log;
extern crate num_cpus;
extern crate provider;
#[macro_use]
extern crate serde_json;
extern crate simplog;
extern crate url;

use std::env;
use std::process::exit;

use clap::{App, AppSettings, Arg, ArgMatches};
use simplog::simplog::SimpleLogger;
use url::Url;

use cli_debug_client::CLIDebugClient;
use flowrlib::coordinator::{Coordinator, Submission};
use flowrlib::debug_client::DebugClient;
use flowrlib::info;
use flowrlib::loader::Loader;
use provider::args::url_from_string;
use provider::content::provider::MetaProvider;

mod runtime;
mod cli_debug_client;

/// The name of the environment variables used to pass command line arguments to the function
/// used to get them.
pub const FLOW_ARGS: &str = "FLOW_ARGS";

const CLI_DEBUG_CLIENT: &dyn DebugClient = &CLIDebugClient {};

// We'll put our errors in an `errors` module, and other modules in
// this crate will `use errors::*;` to get access to everything
// `error_chain!` creates.
mod errors {
    // Create the Error, ErrorKind, ResultExt, and Result types
    error_chain! {}
}

error_chain! {
    foreign_links {
        Provider(provider::errors::Error);
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
    print_versions();
    let flow_manifest_url = get_flow_manifest_url(&matches)?;
    let mut loader = Loader::new();
    let provider = MetaProvider {};

    // Load this runtime's library of native (statically linked) implementations
    loader.add_lib(&provider, runtime::manifest::get_manifest(), "runtime")
        .chain_err(|| "Could not add 'runtime' library to loader")?;

    // If the "native" feature is enabled then load the native flowstdlib if command line arg to do so
    if cfg!(feature = "native") {
        if matches.is_present("native") {
            loader.add_lib(&provider, flowstdlib::get_manifest(), "flowstdlib")
                .chain_err(|| "Could not add 'flowstdlib' library to loader")?;
        }
    }

    let debugger = matches.is_present("debugger");
    let metrics = matches.is_present("metrics");
    let mut coordinator = Coordinator::new(num_threads(&matches, debugger));

    // Load the flow to run from the manifest
    let manifest = loader.load_manifest(&provider, &flow_manifest_url.to_string())
        .chain_err(|| format!("Could not load the flow from manifest: '{}'", flow_manifest_url))?;

    let num_parallel_jobs = num_parallel_jobs(&matches, debugger);

    let debug_client = match debugger {
        false => None,
        true => Some(CLI_DEBUG_CLIENT)
    };

    setup_args(&matches, &manifest.metadata.name);

    let submission = Submission::new(manifest, num_parallel_jobs, metrics, debug_client);

    Ok(coordinator.submit(submission))
}


/*
    Determine the number of threads to use to execute flows, with a default of the number of cores
    in the device, or any override from the command line.

    If debugger=true, then default to 0 threads, unless overridden by an argument
*/
fn num_threads(matches: &ArgMatches, debugger: bool) -> usize {
    match matches.value_of("threads") {
        Some(value) => {
            match value.parse::<i32>() {
                Ok(mut threads) => {
                    if threads < 0 {
                        error!("Minimum number of additional threads is '0', so option of has been overridded to be '0'");
                        threads = 0;
                    }
                    threads as usize
                }
                Err(_) => {
                    error!("Error parsing the value for number of threads '{}'", value);
                    num_cpus::get()
                }
            }
        }
        None => {
            if debugger {
                info!("Due to debugger option being set, number of threads has defaulted to 0");
                0
            } else {
                num_cpus::get()
            }
        }
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
                        error!("Minimum number of parallel jobs is '0', so option of '{}' has been overridded to be '1'",
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

    let app = app.arg(Arg::with_name("flow-manifest")
        .help("the name of the 'flow' manifest file")
        .required(true)
        .index(1))
        .arg(Arg::with_name("debugger")
            .short("d")
            .long("debugger")
            .help("Enable the debugger when running a flow"))
        .arg(Arg::with_name("metrics")
            .short("m")
            .long("metrics")
            .help("Calculate metrics during flow execution and print them out when done"))
        .arg(Arg::with_name("jobs")
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
            .help("Set number of threads to use to execute jobs)"))
        .arg(Arg::with_name("verbosity")
            .short("v")
            .long("verbosity")
            .takes_value(true)
            .value_name("VERBOSITY_LEVEL")
            .help("Set verbosity level for output (trace, debug, info, warn, error (default))"))
        .arg(Arg::with_name("flow-arguments")
            .multiple(true)
            .help("A list of arguments to pass to the flow when executed. Preceed this list with a '-'"));

    #[cfg(feature = "native")]
        let app = app.arg(Arg::with_name("native")
        .short("n")
        .long("native")
        .help("Use native libraries when possible"));

    app.get_matches()
}

/*
    Print some debugging info about the versions of flowr and flowrlib
*/
fn print_versions() {
    info!("'{}' version {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
    info!("'flowrlib' version {}", info::version());
}

/*
    Extract the flow-manifest URL from the command line arguments and return as Url
*/
fn get_flow_manifest_url(matches: &ArgMatches) -> Result<Url> {
    url_from_string(matches.value_of("flow-manifest"))
        .chain_err(|| "Unable to parse the URL of the manifest of the flow to run")
}

/*
    Parse out the command line arguments passed onto the flow itself, and then
    set an anvironment variable with the args. This is then used by the 'args/get' runtime
    function to fetch the args while the flow is running
    arg[0[ - is the name of the flow
    arg[1..] - will be any arguments passed in (after the final '--')
*/
fn setup_args(matches: &ArgMatches, flow_name: &str)  {
    // this will not be unique, but it will be used very soon and removed
    if let Some(flow_args) = matches.values_of("flow-arguments") {
        let mut args: Vec<&str> = flow_args.collect();
        args.insert(0, flow_name);
        env::set_var(FLOW_ARGS, args.join(" "));
        debug!("Setup '{}' with values = '{:?}'", FLOW_ARGS, args);
    }
}
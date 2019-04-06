extern crate clap;
extern crate flowrlib;
extern crate flowstdlib;
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
use flowrlib::coordinator;
use flowrlib::debug_client::DebugClient;
use flowrlib::info;
use flowrlib::loader::Loader;
use simplog::simplog::SimpleLogger;
use url::Url;

use cli_debug_client::CLIDebugClient;
use provider::args::{cwd_as_url, url_from_string};
use provider::content::provider::MetaProvider;

pub mod args;
pub mod stdio;
pub mod file;
mod ilt;
mod cli_debug_client;

pub const FLOW_ARGS_NAME: &str = "FLOW_ARGS";

const CLI_DEBUG_CLIENT: &DebugClient = &CLIDebugClient {};

fn main() -> Result<(), String> {
    let matches = get_matches();
    let url = parse_args(&matches)?;
    let mut loader = Loader::new();
    let provider = MetaProvider {};

    let cwd = cwd_as_url()?;

    // TODO these shoudl come in as library references in the flow and they can be loaded
    // on demand, or reused if already loaded.

    // Load library functions from 'flowr'
    loader.add_lib(&provider, ::ilt::get_ilt(), &cwd.to_string())?;

    // Load standard library functions from flowstdlib
    // For now we are passing in a fake ilt.json file so the basepath for finding wasm files works.
    loader.add_lib(&provider, flowstdlib::ilt::get_ilt(),
                   &format!("{}flowstdlib/ilt.json", cwd.to_string()))?;

    let debugger = matches.is_present("debugger");
    let metrics = matches.is_present("metrics");

    let num_parallel_jobs = num_parallel_jobs(&matches);

    // Load the flow to run from the manifest
    let flow = loader.load_manifest(&provider, &url.to_string())?;

    // run the flow
    coordinator::run(flow, metrics, CLI_DEBUG_CLIENT,
                     debugger, num_parallel_jobs);

    exit(0);
}

/*
    Determine the number of parallel jobs to be run in parallel based on a default of 2 times
    the number of cores in the device, or any override from the command line.
*/
fn num_parallel_jobs(matches: &ArgMatches) -> usize {
    match matches.value_of("jobs") {
        Some(value) => {
            match value.parse::<i32>() {
                Ok(mut jobs) => {
                    if jobs < 1 {
                        error!("Minimum number of parallel jobs is '1', so option of '{}' has been overridded to be '1'",
                        jobs);
                        jobs = 1;
                    }
                    jobs as usize
                },
                Err(_) => {
                    error!("Error parsing the value for number of parallel jobs '{}'", value);
                    2 * num_cpus::get()
                }
            }
        },
        None => 2 * num_cpus::get()
    }
}

/*
    Parse the command line arguments using clap
*/
fn get_matches<'a>() -> ArgMatches<'a> {
    App::new(env!("CARGO_PKG_NAME"))
        .setting(AppSettings::TrailingVarArg)
        .version(env!("CARGO_PKG_VERSION"))
        .arg(Arg::with_name("flow-manifest")
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
        .arg(Arg::with_name("log")
            .short("l")
            .long("log")
            .takes_value(true)
            .value_name("LOG_LEVEL")
            .help("Set log level for output (trace, debug, info, warn, error (default))"))
        .arg(Arg::with_name("flow-arguments")
            .multiple(true))
        .get_matches()
}

/*
    Parse the command line arguments
*/
fn parse_args(matches: &ArgMatches) -> Result<Url, String> {
    // Set anvironment variable with the args
    // this will not be unique, but it will be used very soon and removed
    if let Some(flow_args) = matches.values_of("flow-arguments") {
        let mut args: Vec<&str> = flow_args.collect();
        // arg #0 is the flow/package name
        // TODO fix this to be the name of the flow, not 'flowr'
        args.insert(0, env!("CARGO_PKG_NAME"));
        env::set_var(FLOW_ARGS_NAME, args.join(" "));
        debug!("Setup '{}' with values = '{:?}'", FLOW_ARGS_NAME, args);
    }

    SimpleLogger::init(matches.value_of("log"));

    info!("'{}' version {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
    info!("'flowrlib' version {}\n", info::version());

    url_from_string(matches.value_of("flow-manifest"))
}
extern crate clap;
extern crate flowrlib;
extern crate flowstdlib;
#[macro_use]
extern crate log;
extern crate provider;
#[macro_use]
extern crate serde_json;
extern crate simplog;
extern crate url;

use std::env;
use std::io;
use std::io::Write;
use std::process::exit;

use clap::{App, AppSettings, Arg, ArgMatches};
use flowrlib::debug_client::DebugClient;
use flowrlib::info;
use flowrlib::loader::Loader;
use flowrlib::runlist::run;
use simplog::simplog::SimpleLogger;
use url::Url;

use provider::args::{cwd_as_url, url_from_string};
use provider::content::provider::MetaProvider;

pub mod args;
pub mod stdio;
pub mod file;
mod ilt;

pub const FLOW_ARGS_NAME: &str = "FLOW_ARGS";

struct CLIDebugClient {}

/*
    Implement a client for the debugger that reads and writes to standard input and output
*/
impl DebugClient for CLIDebugClient {
    fn display(&self, output: &str) {
        print!("{}", output);
        io::stdout().flush().unwrap();
    }

    fn read_input(&self, input: &mut String) -> io::Result<usize> {
        io::stdin().read_line(input)
    }
}

const CLI_DEBUG_CLIENT: &DebugClient = &CLIDebugClient{};

fn main() -> Result<(), String> {
    let matches = get_matches();
    let url = parse_args(&matches)?;
    let mut loader = Loader::new();
    let provider = MetaProvider {};

    let cwd = cwd_as_url()?;

    // Load library functions from 'flowr'
    loader.add_lib(&provider, ::ilt::get_ilt(), &cwd.to_string())?;

    // Load standard library functions from flowstdlib
    // For now we are passing in a fake ilt.json file so the basepath for finding wasm files works.
    loader.add_lib(&provider, flowstdlib::ilt::get_ilt(),
                   &format!("{}flowstdlib/ilt.json", cwd.to_string()))?;

    // Load the list of processes to be run from the manifest
    loader.load_manifest(&provider, &url.to_string())?;

    let debugger = matches.is_present("debugger");
    let metrics = matches.is_present("metrics");

    // run the set of flow processes
    run(loader.processes, metrics, CLI_DEBUG_CLIENT, debugger);

    exit(0);
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
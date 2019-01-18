extern crate clap;
extern crate curl;
extern crate flowrlib;
extern crate flowstdlib;
#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_json;
extern crate simpath;
extern crate simplog;
extern crate tempdir;
extern crate url;

use std::env;
use std::process::exit;

use clap::{App, AppSettings, Arg, ArgMatches};
use flowrlib::execution::execute;
use flowrlib::info;
use flowrlib::loader::Loader;
use simplog::simplog::SimpleLogger;

mod provider;
pub mod args;
pub mod stdio;
pub mod file;
mod manifest;

pub const FLOW_ARGS_NAME: &str = "FLOW_ARGS";

fn main() -> Result<(), String> {
    let path = parse_args( get_matches())?;
    let mut loader = Loader::new();
    let flowr_provider = provider::FlowrProvider{};

    loader.load_lib(::manifest::get_manifest());
    loader.load_lib(flowstdlib::manifest::get_manifest());


    let runnables = loader.load_flow(&flowr_provider, &path)?;

    execute(runnables);

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
fn parse_args(matches: ArgMatches) -> Result<String, String> {
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

    match matches.value_of("flow-manifest") {
        Some(path) => Ok(path.to_string()),
        None => Err("No flow manifest filename specified".to_string())
    }
}


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
use std::process::exit;

use clap::{App, AppSettings, Arg, ArgMatches};
use flowrlib::execution::execute;
use flowrlib::info;
use flowrlib::loader::Loader;
use simplog::simplog::SimpleLogger;
use url::Url;

use provider::content::args::url_from_string;
use provider::content::provider::MetaProvider;

pub mod args;
pub mod stdio;
pub mod file;
mod manifest;

pub const FLOW_ARGS_NAME: &str = "FLOW_ARGS";

fn main() -> Result<(), String> {
    let url = parse_args( get_matches())?;
    let mut loader = Loader::new();
    let flowr_provider = MetaProvider{};

    // Load standard library functions we always want - flowr (for environment) and flowstdlib
    loader.load_lib(::manifest::get_manifest());
    loader.load_lib(flowstdlib::manifest::get_manifest());

    let runnables = loader.load_flow(&flowr_provider, &url)?;

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
fn parse_args(matches: ArgMatches) -> Result<Url, String> {
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

    url_from_string( matches.value_of("flow-manifest"))
}


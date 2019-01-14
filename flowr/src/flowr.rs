extern crate clap;
extern crate curl;
extern crate flowrlib;
#[macro_use]
extern crate log;
extern crate serde_json;
extern crate simpath;
extern crate simplog;
extern crate tempdir;
extern crate url;

use std::env;
use std::io::{Error, ErrorKind};
use std::path::PathBuf;
use std::process::exit;

use clap::{App, AppSettings, Arg, ArgMatches};
use flowrlib::execution::execute;
use flowrlib::info;
use simplog::simplog::SimpleLogger;

mod loader;

fn main() -> Result<(), Error> {
    let path = parse_args( get_matches())?;

    let runnables = loader::load_flow(&path)?;

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
        .arg(Arg::with_name("log")
            .short("l")
            .long("log")
            .takes_value(true)
            .value_name("LOG_LEVEL")
            .help("Set log level for output (trace, debug, info, warn, error (default))"))
        .arg(Arg::with_name("FLOW")
            .help("the name of the compiled 'flow' file")
            .required(false)
            .index(1))
        .arg(Arg::with_name("flow_args")
            .multiple(true))
        .get_matches()
}

/*
    Parse the command line arguments
*/
fn parse_args(matches: ArgMatches) -> Result<PathBuf, Error> {
    // Set anvironment variable with the args
    // this will not be unique, but it will be used very soon and removed
    if let Some(flow_args) = matches.values_of("flow_args") {
        let mut args: Vec<&str> = flow_args.collect();
        args.insert(0, env!("CARGO_PKG_NAME")); // arg #0 is the flow/package name
        env::set_var("FLOW_ARGS", args.join(" "));
    }

    SimpleLogger::init(matches.value_of("log"));

    info!("'{}' version {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
    info!("'flowrlib' version {}\n", info::version());

    match matches.value_of("FLOW") {
        Some(path) => Ok(PathBuf::from(path)),
        None => Err(Error::new(ErrorKind::Other, "No flow filename specified"))
    }
}


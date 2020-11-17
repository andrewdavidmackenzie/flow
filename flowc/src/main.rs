// TODO #![deny(missing_docs)]
#![warn(clippy::unwrap_used)]
//! `flowc` the the "flow compiler" that takes a hierarchical description of flows
//! and functions and compiles it into a network of functions in a `Manifest` file
//! for execution by `flowr` or other flow runtimes.
//!
//! Execute `flowc` or `flowc --help` or `flowc -h` at the comment line for a
//! description of the command line options.

#[macro_use]
extern crate error_chain;

use std::env;
use std::path::PathBuf;
use std::process::exit;

use clap::{App, AppSettings, Arg, ArgMatches};
use log::debug;
use simplog::simplog::SimpleLogger;
use url::Url;

use flowclib::info;
use provider::args::url_from_string;
use provider::content::provider::MetaProvider;

use crate::flow_compile::compile_and_execute_flow;
use crate::lib_build::build_lib;

mod source_arg;
mod lib_build;
mod flow_compile;
mod compile_wasm;

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
        Compiler(flowclib::errors::Error);
        Io(std::io::Error);
    }
}

pub struct Options {
    lib: bool,
    url: Url,
    flow_args: Vec<String>,
    dump: bool,
    skip_execution: bool,
    debug_symbols: bool,
    provided_implementations: bool,
    output_dir: PathBuf,
    stdin_file: Option<String>
}

fn main() {
    match run() {
        Err(ref e) => {
            println!("error: {}", e);

            for e in e.iter().skip(1) {
                println!("caused by: {}", e);
            }

            // The backtrace is generated if env var `RUST_BACKTRACE` is set to `1` or `full`
            if let Some(backtrace) = e.backtrace() {
                println!("backtrace: {:?}", backtrace);
            }

            exit(1);
        }
        Ok(msg) => {
            if !msg.is_empty() {
                println!("{}", msg);
            }
            exit(0)
        }
    }
}

/*
    run the loader to load the process and (optionally) compile, generate code and run the flow.
    Return either an error string if anything goes wrong or
    a message to display to the user if all went OK
*/
fn run() -> Result<String> {
    let options = parse_args(get_matches())?;

    let provider = &MetaProvider {};

    if options.lib {
        build_lib(&options, provider).chain_err(|| "Could not build library")
    } else {
        compile_and_execute_flow(&options, provider)
            .chain_err(|| format!("Could not compile and execute the flow '{}'", &options.url))
    }
}

/*
    Parse the command line arguments using clap
*/
fn get_matches<'a>() -> ArgMatches<'a> {
    App::new(env!("CARGO_PKG_NAME"))
        .setting(AppSettings::TrailingVarArg)
        .version(env!("CARGO_PKG_VERSION"))
        .arg(Arg::with_name("skip")
            .short("s")
            .long("skip")
            .help("Skip execution of flow"))
        .arg(Arg::with_name("lib")
            .short("l")
            .long("lib")
            .help("Compile a flow library"))
        .arg(Arg::with_name("dump")
            .short("d")
            .long("dump")
            .help("Dump the flow to .dump files after loading it"))
        .arg(Arg::with_name("symbols")
            .short("g")
            .long("symbols")
            .help("Generate debug symbols (like process names and full routes)"))
        .arg(Arg::with_name("provided")
            .short("p")
            .long("provided")
            .help("Provided function implementations should NOT be compiled from source"))
        .arg(Arg::with_name("output")
            .short("o")
            .long("output")
            .takes_value(true)
            .value_name("OUTPUT_DIR")
            .help("Specify the output directory for generated manifest"))
        .arg(Arg::with_name("verbosity")
            .short("v")
            .long("verbosity")
            .takes_value(true)
            .value_name("VERBOSITY_LEVEL")
            .help("Set verbosity level for output (trace, debug, info, warn, error (default))"))
        .arg(Arg::with_name("stdin")
            .short("i")
            .long("stdin")
            .takes_value(true)
            .value_name("STDIN_FILENAME")
            .help("Read STDIN from the named file"))
        .arg(Arg::with_name("FLOW")
            .help("the name of the 'flow' definition file to compile")
            .required(false)
            .index(1))
        .arg(Arg::with_name("flow_args")
            .help("Arguments that will get passed onto the flow if it is executed")
            .multiple(true))
        .get_matches()
}

/*
    Parse the command line arguments
*/
fn parse_args(matches: ArgMatches) -> Result<Options> {
    let mut flow_args: Vec<String> = vec!();
    if let Some(fargs) = matches.values_of("flow_args") {
        flow_args = fargs.map(|a| a.to_string()).collect();
    }

    SimpleLogger::init_prefix(matches.value_of("verbosity"), false);

    debug!("'{}' version {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
    debug!("'flowclib' version {}", info::version());

    let cwd = env::current_dir().chain_err(|| "Could not get current working directory value")?;
    let cwd_url =     Url::from_directory_path(cwd)
        .map_err(|_|"Could not form a Url for the current working directory")?;

    let url = url_from_string(&cwd_url, matches.value_of("FLOW"))
        .chain_err(|| "Could not create a url for flow from the 'FLOW' command line parameter")?;

    let output_dir = source_arg::get_output_dir(&url, matches.value_of("output"))
        .chain_err(|| "Could not get or create the output directory")?;

    Ok(Options {
        lib: matches.is_present("lib"),
        url,
        flow_args,
        dump: matches.is_present("dump"),
        skip_execution: matches.is_present("skip"),
        debug_symbols: matches.is_present("symbols"),
        provided_implementations: matches.is_present("provided"),
        output_dir,
        stdin_file: matches.value_of("stdin").map(String::from)
    })
}

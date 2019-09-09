extern crate clap;
#[macro_use]
extern crate error_chain;
extern crate flowclib;
extern crate flowrlib;
#[macro_use]
extern crate log;
extern crate provider;
extern crate serde_json;
extern crate simpath;
extern crate simplog;
extern crate tempdir;
extern crate url;

use std::path::PathBuf;

use clap::{App, AppSettings, Arg, ArgMatches};
use simplog::simplog::SimpleLogger;
use url::Url;

use flowclib::info;
use provider::args::url_from_string;
use provider::content::provider::MetaProvider;

use crate::flow_compile::compile_flow;
use crate::lib_build::build_lib;

mod source_arg;
mod lib_build;
mod flow_compile;
mod compile_wasm;

// We'll put our errors in an `errors` module, and other modules in
// this crate will `use errors::*;` to get access to everything
// `error_chain!` creates.
pub mod errors {
    // Create the Error, ErrorKind, ResultExt, and Result types
    error_chain! {}
}

error_chain! {
    types {
        Error, ErrorKind, ResultExt, Result;
    }

    foreign_links {
        Provider(provider::errors::Error);
        Compiler(flowclib::errors::Error);
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

            ::std::process::exit(1);
        }
        Ok(_) => {}
    }
}

/*
    run the loader to load the process and (optionally) compile, generate code and run the flow.
    Return either an error string if anything goes wrong or
    a message to display to the user if all went OK
*/
fn run() -> Result<String> {
    let (lib, url, args, dump, skip_generation, debug_symbols,
        provided_implementations, base_dir, release)
        = parse_args(get_matches())?;

    let provider = &MetaProvider {};

    if lib {
        build_lib(url, provided_implementations, base_dir, provider, release)
            .expect("Could not build library");
    } else {
        compile_flow(url, args, dump, skip_generation, debug_symbols, provided_implementations, base_dir, provider, release)
            .expect("Could not compile flow");
    }

    Ok("flowc completed".into())
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
            .help("Skip manifest generation and running of flow"))
        .arg(Arg::with_name("release")
            .short("r")
            .long("release")
            .help("Build supplied and library implementations with release profile"))
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
        .arg(Arg::with_name("FLOW")
            .help("the name of the 'flow' definition file to compile")
            .required(false)
            .index(1))
        .arg(Arg::with_name("flow_args")
            .multiple(true))
        .get_matches()
}

/*
    Parse the command line arguments
*/
fn parse_args(matches: ArgMatches) -> Result<(bool, Url, Vec<String>, bool, bool, bool, bool, PathBuf, bool)> {
    let mut args: Vec<String> = vec!();
    if let Some(flow_args) = matches.values_of("flow_args") {
        args = flow_args.map(|a| a.to_string()).collect();
    }

    SimpleLogger::init(matches.value_of("verbosity"));

    debug!("'{}' version {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
    debug!("'flowclib' version {}", info::version());

    let url = url_from_string(matches.value_of("FLOW"))
        .chain_err(|| "Could not create a url for flow from the 'FLOW' command line parameter")?;

    let lib = matches.is_present("lib");
    let dump = matches.is_present("dump");
    let skip_generation = matches.is_present("skip");
    let release = matches.is_present("release");
    let debug_symbols = matches.is_present("symbols");
    let provided_implementations = matches.is_present("provided");
    let out_dir_option = matches.value_of("OUTPUT_DIR");
    let output_dir = source_arg::get_output_dir(&url, out_dir_option)
        .chain_err(|| "Could not get or create the output directory")?;

    Ok((lib, url, args, dump, skip_generation, debug_symbols, provided_implementations, output_dir, release))
}

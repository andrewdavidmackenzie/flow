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
use std::io;
use std::path::{Path, PathBuf};
use std::process::exit;

use clap::{App, AppSettings, Arg, ArgMatches};
use log::{debug, info};
use simpath::Simpath;
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
    stdin_file: Option<String>,
    lib_dirs: Vec<String>
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
    For the lib provider, libraries maybe installed in multiple places in the file system.
    In order to find the content, a FLOW_LIB_PATH environment variable can be configured with a
    list of directories in which to look for the library in question.

    Using the "FLOW_LIB_PATH" environment variable attempt to locate the library's root folder
    in the file system.
 */
pub fn set_lib_search_path(lib_dirs: &[String]) -> Result<Simpath> {
    let mut lib_search_path = Simpath::new("FLOW_LIB_PATH");

    if env::var("FLOW_LIB_PATH").is_ok(){
        info!("Library Search Path initialized from 'FLOW_LIB_PATH'");
    }
    else {
        info!("'FLOW_LIB_PATH' not set so initializing Library Search Path from Project directories");
        let project_root = Path::new(env!("CARGO_MANIFEST_DIR")).parent()
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Could not get Project root dir"))?;
        let project_root_str = project_root.to_str().chain_err(|| "Unable to convert project_root Path to String")?;
        lib_search_path.add_directory(project_root_str);
        info!("Directory '{}' added to the Library Search Path", project_root_str);

        let runtime_parent = project_root.join("flowr/src/lib");
        let runtime_parent_str = runtime_parent.to_str().chain_err(|| "Unable to convert runtime Path to String")?;
        lib_search_path.add_directory(runtime_parent_str);
        info!("Directory '{}' added to the Library Search Path", runtime_parent_str);
    }

    // Add any library search directories specified via the command line
    for dir in lib_dirs {
        lib_search_path.add_directory(dir);
        info!("Directory '{}' specified on command line via '-L' added to the Library Search Path", dir);
    }

    Ok(lib_search_path)
}

/*
    run the loader to load the process and (optionally) compile, generate code and run the flow.
    Return either an error string if anything goes wrong or
    a message to display to the user if all went OK
*/
fn run() -> Result<String> {
    let options = parse_args(get_matches())?;

    let lib_search_path = set_lib_search_path(&options.lib_dirs)?;

    let provider = &MetaProvider::new(lib_search_path);

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
        .arg(Arg::with_name("lib_dir")
            .short("L")
            .long("libdir")
            .number_of_values(1)
            .multiple(true)
            .value_name("LIB_DIR")
            .help("Add the directory or Url to the Library Search path"))
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
    if let Some(args) = matches.values_of("flow_args") {
        flow_args = args.map(|a| a.to_string()).collect();
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

    let lib_dirs = if matches.is_present("lib_dir") {
        matches.values_of("lib_dir")
            .chain_err(|| "Could not get the list of 'LIB_DIR' options specified")?
            .map(|s| s.to_string()).collect()
    } else {
        vec!()
    };

    Ok(Options {
        lib: matches.is_present("lib"),
        url,
        flow_args,
        dump: matches.is_present("dump"),
        skip_execution: matches.is_present("skip"),
        debug_symbols: matches.is_present("symbols"),
        provided_implementations: matches.is_present("provided"),
        output_dir,
        stdin_file: matches.value_of("stdin").map(String::from),
        lib_dirs
    })
}

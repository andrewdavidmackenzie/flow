#![deny(missing_docs)]
#![warn(clippy::unwrap_used)]
#![allow(clippy::result_large_err)]
//! `flowc` the the "flow compiler" that takes a hierarchical description of flows
//! and functions and compiles it into a network of functions in a `Manifest` file
//! for execution by `flowr` or other flow runtimes.
//!
//! Execute `flowc` or `flowc --help` or `flowc -h` at the comment line for a
//! description of the command line options.

use std::env;
use std::path::PathBuf;
use std::process::exit;

use clap::{Arg, ArgMatches, Command};
use colored::*;
use log::{debug, error, info, warn};
use simpath::Simpath;
use simplog::SimpleLogger;
use url::Url;

use errors::*;
use flowclib::info;
use flowcore::meta_provider::MetaProvider;
use flowcore::url_helper::url_from_string;
use lib_build::build_lib;

use crate::flow_compile::compile_and_execute_flow;

/// We'll put our errors in an `errors` module, and other modules in this crate will
/// `use crate::errors::*;` to get access to everything `error_chain` creates.
pub mod errors;

/// `lib_build` module is used to compile a flow library from source
pub mod lib_build;

mod flow_compile;
mod source_arg;

/// `Options` struct gathers information from the parsing of the command line options
/// to be used to configure execution
pub struct Options {
    lib: bool,
    source_url: Url,
    flow_args: Vec<String>,
    tables_dump: bool,
    graphs: bool,
    execution_metrics: bool,
    wasm_execution: bool,
    compile_only: bool,
    debug_symbols: bool,
    provided_implementations: bool,
    output_dir: PathBuf,
    stdin_file: Option<String>,
    lib_dirs: Vec<String>,
    native_only: bool,
    context_root: Option<PathBuf>,
    verbosity: Option<String>,
    optimize: bool,
}

fn main() {
    match run() {
        Err(ref e) => {
            error!("{}: {}", "error".red(), e);

            for e in e.iter().skip(1) {
                error!("caused by: {}", e);
            }

            // The backtrace is generated if env var `RUST_BACKTRACE` is set to `1` or `full`
            if let Some(backtrace) = e.backtrace() {
                error!("backtrace: {:?}", backtrace);
            }

            exit(1);
        }
        Ok(_) => exit(0)
    }
}

/// For the lib provider, libraries maybe installed in multiple places in the file system.
/// In order to find the content, a FLOW_LIB_PATH environment variable can be configured with a
/// list of directories in which to look for the library in question.
pub fn get_lib_search_path(search_path_additions: &[String]) -> Result<Simpath> {
    let mut lib_search_path = Simpath::new_with_separator("FLOW_LIB_PATH", ',');

    if env::var("FLOW_LIB_PATH").is_err() && search_path_additions.is_empty() {
        warn!(
            "'FLOW_LIB_PATH' env var not set and no LIB_DIRS options supplied. Library references may not be found."
        );
    }

    for addition in search_path_additions {
        lib_search_path.add(addition);
        info!("'{}' added to the Library Search Path", addition);
    }

    Ok(lib_search_path)
}

/*
    run the loader to load the process and (optionally) compile, generate code and run the flow.
    Return either an error string if anything goes wrong or
    a message to display to the user if all went OK
*/
fn run() -> Result<()> {
    let options = parse_args(get_matches())?;
    let lib_search_path = get_lib_search_path(&options.lib_dirs)?;
    let context_root = options.context_root.clone().unwrap_or_else(|| PathBuf::from(""));
    let provider = &MetaProvider::new(lib_search_path, context_root);

    if options.lib {
        build_lib(&options, provider).chain_err(|| "Could not build library")
    } else {
        compile_and_execute_flow(&options, provider)
    }
}

/*
    Parse the command line arguments using clap
*/
fn get_matches() -> ArgMatches {
    let app = Command::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"));

    #[cfg(feature = "debugger")]
    let app = app.arg(
        Arg::new("debug")
            .short('d')
            .long("debug")
            .action(clap::ArgAction::SetTrue)
            .help("Generate symbols for debugging. If executing the flow, do so with the debugger"),
    );

    let app = app
        .arg(
            Arg::new("compile")
                .short('c')
                .long("compile")
                .action(clap::ArgAction::SetTrue)
                .help("Compile the flow and implementations, but do not execute"),
        )
        .arg(
            Arg::new("context_root")
                .short('C')
                .long("context_root")
                .num_args(0..1)
                .number_of_values(1)
                .value_name("CONTEXT_DIRECTORY")
                .help("Set the directory to use as the root dir for context function definitions"),
        )
        .arg(
            Arg::new("lib")
                .short('l')
                .long("lib")
                .action(clap::ArgAction::SetTrue)
                .help("Compile a flow library"),
        )
        .arg(
            Arg::new("native")
                .short('n')
                .long("native")
                .action(clap::ArgAction::SetTrue)
                .help("Compile only native (not wasm) implementations when compiling a library")
                .requires("lib"),
        )
        .arg(
            Arg::new("lib_dir")
                .short('L')
                .long("libdir")
                .num_args(0..)
                .number_of_values(1)
                .value_name("LIB_DIR|BASE_URL")
                .help("Add a directory or base Url to the Library Search path"),
        )
        .arg(
            Arg::new("tables")
                .short('t')
                .long("tables")
                .action(clap::ArgAction::SetTrue)
                .help("Write flow and compiler tables to .dump and .dot files"),
        )
        .arg(
            Arg::new("graphs")
                .short('g')
                .long("graphs")
                .action(clap::ArgAction::SetTrue)
                .help("Create .dot files for graphs then generate SVGs with 'dot' command (if available)"),
        )
        .arg(
            Arg::new("metrics")
                .short('m')
                .long("metrics")
                .conflicts_with("compile")
                .action(clap::ArgAction::SetTrue)
                .help("Show flow execution metrics when execution ends"),
        )
        .arg(
            Arg::new("wasm")
                .short('w')
                .long("wasm")
                .action(clap::ArgAction::SetTrue)
                .conflicts_with("compile")
                .help("Use wasm library implementations when executing flow"),
        )
        .arg(
            Arg::new("optimize")
                .short('O')
                .long("optimize")
                .action(clap::ArgAction::SetTrue)
                .help("Optimize generated output (flows and wasm)"),
        )
        .arg(
            Arg::new("provided")
                .short('p')
                .long("provided")
                .action(clap::ArgAction::SetTrue)
                .help("Provided function implementations should NOT be compiled from source"),
        )
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .num_args(0..1)
                .number_of_values(1)
                .value_name("OUTPUT_DIR")
                .help("Specify the output directory for generated manifest"),
        )
        .arg(
            Arg::new("verbosity")
                .short('v')
                .long("verbosity")
                .num_args(0..1)
                .number_of_values(1)
                .value_name("VERBOSITY_LEVEL")
                .help("Set verbosity level for output (trace, debug, info, warn, error (default))"),
        )
        .arg(
            Arg::new("stdin")
                .short('i')
                .long("stdin")
                .num_args(0..1)
                .number_of_values(1)
                .value_name("STDIN_FILENAME")
                .help("Read STDIN from the named file"),
        )
        .arg(
            Arg::new("source_url")
                .num_args(1)
                .help("path or url for the flow or library to compile")
        )
        .arg(
            Arg::new("flow_args")
                .num_args(0..)
                .trailing_var_arg(true)
                .help("List of arguments get passed to the flow when executed")
        );

    app.get_matches()
}

/*
    Parse the command line arguments
*/
fn parse_args(matches: ArgMatches) -> Result<Options> {
    let verbosity = matches.get_one::<String>("verbosity").map(|s| s.as_str());
    SimpleLogger::init_prefix(verbosity, false);

    debug!(
        "'{}' version {}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    );
    debug!("'flowclib' version {}", info::version());

    let cwd = env::current_dir().chain_err(|| "Could not get current working directory value")?;
    let cwd_url = Url::from_directory_path(cwd)
        .map_err(|_| "Could not form a Url for the current working directory")?;

    let url = url_from_string(&cwd_url,
                              matches.get_one::<String>("source_url")
                                  .map(|s| s.as_str()))
        .chain_err(|| "Could not create a url for flow from the 'FLOW' command line parameter")?;

    let output_dir = source_arg::get_output_dir(&url,
                                                matches.get_one::<String>("output")
                                                    .map(|s| s.as_str()))
        .chain_err(|| "Could not get or create the output directory")?;

    let lib_dirs = if matches.contains_id("lib_dir") {
        matches
            .get_many::<String>("lib_dir")
            .chain_err(|| "Could not get the list of 'LIB_DIR' options specified")?
            .map(|s| s.to_string())
            .collect()
    } else {
        vec![]
    };

    let context_root = if matches.contains_id("context_root") {
        let root_string = matches.get_one::<String>("context_root")
            .ok_or("Could not get the 'CONTEXT_DIRECTORY' option specified")?;
        let root = PathBuf::from(root_string);
        Some(root.canonicalize()?)
    } else {
        None
    };

    let flow_args = match matches.get_many::<String>("flow_args") {
        Some(strings) => strings.map(|s| s.to_string()).collect(),
        None => vec![]
    };

    Ok(Options {
        lib: matches.get_flag("lib"),
        source_url: url,
        flow_args,
        tables_dump: matches.get_flag("tables"),
        graphs: matches.get_flag("graphs"),
        wasm_execution: matches.get_flag("wasm"),
        execution_metrics: matches.get_flag("metrics"),
        compile_only: matches.get_flag("compile"),
        debug_symbols: matches.get_flag("debug"),
        provided_implementations: matches.get_flag("provided"),
        output_dir,
        stdin_file: matches.get_one::<String>("stdin").map(|s| s.to_string()),
        lib_dirs,
        native_only: matches.get_flag("native"),
        context_root,
        verbosity: verbosity.map(|s| s.to_string()),
        optimize: matches.get_flag("optimize")
    })
}

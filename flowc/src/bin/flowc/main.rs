#![deny(missing_docs)]
#![warn(clippy::unwrap_used)]
#![allow(clippy::result_large_err)]
//! `flowc` is a "flow compiler" that takes a hierarchical description of a flow
//! using [flow definitions][flowcore::model::flow_definition::FlowDefinition],
//! [function definitions][flowcore::model::function_definition::FunctionDefinition] and
//! [process references][flowcore::model::process_reference::ProcessReference] and compiles it down
//! into a graph of [runtime functions][flowcore::model::runtime_function::RuntimeFunction]
//! in a [flow manifest][flowcore::model::flow_manifest::FlowManifest] for execution by a flow runner.
//!
//! Run `flowc --help` or `flowc -h` at the command line for a
//! description of the command line options.

use core::str::FromStr;
use std::{env, fs};
use std::path::{Path, PathBuf};
use std::process::exit;

use clap::{Arg, ArgMatches, Command};
use env_logger::Builder;
use log::{debug, error, info, LevelFilter, warn};
use serde_derive::Deserialize;
use simpath::Simpath;
use url::Url;

use errors::*;
use flowcore::meta_provider::MetaProvider;
use flowcore::url_helper::url_from_string;
use flowrclib::info;
use lib_build::build_lib;

use crate::flow_compile::compile_and_execute_flow;

/// Contains [Error] types that other modules in this crate will
/// `use crate::errors::*;` to get access to everything `error_chain` creates.
pub mod errors;

mod flow_compile;

/// used to compile a flow library from source
pub mod lib_build;

mod source_arg;

/// information from the parsing of the command line options, to be used to configure execution
pub struct Options {
    source_url: Url,
    flow_args: Vec<String>,
    graphs: bool,
    execution_metrics: bool,
    wasm_execution: bool,
    compile_only: bool,
    debug_symbols: bool,
    provided_implementations: bool,
    output_dir: Option<String>,
    stdin_file: Option<String>,
    lib_dirs: Vec<String>,
    native_only: bool,
    context_root: Option<PathBuf>,
    verbosity: Option<String>,
    optimize: bool,
}

#[derive(Deserialize)]
struct RunnerSpec {
    name: String
}

fn main() {
    match run() {
        Err(ref e) => {
            error!("{e}");
            for e in e.iter().skip(1) {
                error!("caused by: {e}");
            }

            // The backtrace is generated if env var `RUST_BACKTRACE` is set to `1` or `full`
            if let Some(backtrace) = e.backtrace() {
                error!("backtrace: {backtrace:?}");
            }

            exit(1);
        }
        Ok(_) => exit(0),
    }
}

/// For the lib provider, libraries maybe installed in multiple places in the file system.
/// In order to find the content, a `FLOW_LIB_PATH` environment variable can be configured with a
/// list of directories in which to look for the library in question.
fn get_lib_search_path(search_path_additions: &[String]) -> Result<Simpath> {
    let mut lib_search_path = Simpath::new_with_separator("FLOW_LIB_PATH", ',');

    for addition in search_path_additions {
        lib_search_path.add(addition);
        info!("'{}' added to the Library Search Path", addition);
    }

    if lib_search_path.is_empty() {
        warn!("'$FLOW_LIB_PATH' not set and no LIB_DIRS supplied. Libraries may not be found.");
    }

    Ok(lib_search_path)
}

// Load a `RunnerSpec` from the context at `context_root`
fn load_runner_spec(context_root: &Path) -> Result<RunnerSpec> {
    let path = context_root.join("runner.toml");
    let runner_spec = fs::read_to_string(path)?;
    Ok(toml::from_str(&runner_spec)?)
}

// Determine if the specified source url points to a library, by detecting
// - it points to a lib.toml file
// - it points to a directory that contains a lib.toml file
fn is_a_library(url: &Url) -> Result<bool> {
    let mut path = url.to_file_path()
        .map_err(|_| "Could not get local file path for Url")?;

    if path.exists() && path.is_dir() {
        path = path.join("lib.toml");
    }

    if path.exists() && path.is_file() {
        let file_name = path.file_name().ok_or("Could not get file name")?;
        if file_name == "lib.toml" {
            return Ok(true);
        }
    }

    Ok(false)
}

/*
    run the loader to load the process and (optionally) compile, generate code and run the flow.
    Return either an error string if anything goes wrong or
    a message to display to the user if all went OK
*/
fn run() -> Result<()> {
    let options = parse_args(get_matches())?;
    let mut lib_search_path = get_lib_search_path(&options.lib_dirs)?;

    if is_a_library(&options.source_url)? {
        let output_dir = source_arg::get_output_dir(&options.source_url,
                                                    &options.output_dir, true)
            .chain_err(|| "Could not get the output directory")?;

        // Add the parent of the output_dir to the search path so compiler can find internal
        // references functions and flows during the lib build process
        let output_dir_parent = output_dir.parent()
            .ok_or("Could not get parent of output dir")?
            .to_string_lossy();
        lib_search_path.add(&output_dir_parent);
        let provider = &MetaProvider::new(lib_search_path, PathBuf::default());
        build_lib(&options, provider, output_dir).chain_err(|| "Could not build library")
    } else {
        let output_dir = source_arg::get_output_dir(&options.source_url,
                                                    &options.output_dir, false)
            .chain_err(|| "Could not get the output directory")?;

        let context_root = options.context_root.as_ref()
            .ok_or("Context Root was not specified")?;
        let provider = &MetaProvider::new(lib_search_path, context_root.clone());
        let runner_spec = load_runner_spec(context_root)?;
        compile_and_execute_flow(&options, provider, runner_spec.name, output_dir)
    }
}

// Parse the command line arguments using clap
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
            Arg::new("native")
                .short('n')
                .long("native")
                .action(clap::ArgAction::SetTrue)
                .help("Compile only native (not wasm) implementations when compiling"),
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
                .help("Specify a non-default directory for generated output. \
                Default is $HOME/.flow/lib/{lib_name} for a library."),
        )
        .arg(
            Arg::new("verbosity")
                .short('v')
                .long("verbosity")
                .num_args(0..1)
                .number_of_values(1)
                .value_name("VERBOSITY_LEVEL")
                .help("Set verbosity level for output (trace, debug, info, warn, error (default), off)"),
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

// Parse the command line arguments
fn parse_args(matches: ArgMatches) -> Result<Options> {
    let default = String::from("error");
    let verbosity_option = matches.get_one::<String>("verbosity");
    let verbosity = verbosity_option.unwrap_or(&default);
    let level = LevelFilter::from_str(verbosity).unwrap_or(LevelFilter::Error);
    let mut builder = Builder::from_default_env();
    builder.filter_level(level).init();

    debug!(
        "'{}' version {}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    );
    debug!("'flowrclib' version {}", info::version());

    let cwd = env::current_dir()
        .chain_err(|| "Could not get the current working directory")?;
    let cwd_url = Url::from_directory_path(cwd)
        .map_err(|_| "Could not form a Url for the current working directory")?;

    let source_url = url_from_string(&cwd_url,
                                     matches.get_one::<String>("source_url")
                                  .map(|s| s.as_str()))
        .chain_err(|| "Could not create a url for flow from the 'FLOW' command line parameter")?;

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
        source_url,
        flow_args,
        graphs: matches.get_flag("graphs"),
        wasm_execution: matches.get_flag("wasm"),
        execution_metrics: matches.get_flag("metrics"),
        compile_only: matches.get_flag("compile"),
        debug_symbols: matches.get_flag("debug"),
        provided_implementations: matches.get_flag("provided"),
        output_dir: matches.get_one::<String>("output").map(|s| s.to_string()),
        stdin_file: matches.get_one::<String>("stdin").map(|s| s.to_string()),
        lib_dirs,
        native_only: matches.get_flag("native"),
        context_root,
        verbosity: verbosity_option.map(|s| s.to_string()),
        optimize: matches.get_flag("optimize")
    })
}

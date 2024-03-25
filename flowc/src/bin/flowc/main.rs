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
use std::env;
use std::path::PathBuf;
use std::process::exit;

use clap::{Arg, ArgMatches, Command};
use env_logger::Builder;
use log::{debug, error, info, LevelFilter};
use serde_derive::Deserialize;
use simpath::Simpath;
use url::Url;

use errors::{Result, ResultExt};
use flowcore::meta_provider::MetaProvider;
use flowcore::url_helper::url_from_string;
use flowrclib::info;
use lib_build::build_lib;

use crate::flow_compile::compile_and_execute_flow;
use crate::lib_build::build_runner;
use crate::source_arg::{CompileType, default_runner_dir, load_runner_spec};

mod errors;
mod flow_compile;
mod lib_build;
mod source_arg;

#[allow(clippy::struct_excessive_bools)]
pub(crate) struct Options {
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
    runner_name: Option<String>,
    verbosity: Option<String>,
    optimize: bool,
}

#[derive(Deserialize)]
pub(crate) struct RunnerSpec {
    pub(crate) name: String
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
        Ok(()) => exit(0),
    }
}

// For the lib provider, libraries maybe installed in multiple places in the file system.
// In order to find the content, a `FLOW_LIB_PATH` environment variable can be configured with a
// list of directories in which to look for the library in question.
fn get_lib_search_path(search_path_additions: &[String]) -> Simpath {
    let mut lib_search_path = Simpath::new_with_separator("FLOW_LIB_PATH", ',');

    for addition in search_path_additions {
        lib_search_path.add(addition);
        info!("'{}' added to the Library Search Path", addition);
    }

    if lib_search_path.is_empty() {
        let home_dir = env::var("HOME")
            .unwrap_or_else(|_| "Could not get $HOME".to_string());
        lib_search_path.add(&format!("{home_dir}/.flow/lib"));
    }

    lib_search_path
}

// Determine the type of compile to be done
// - it points to a lib.toml file
// - it points to a directory that contains a lib.toml file
// - it points to a runner.toml file
// - it points to a directory that contains a runner.toml file
fn compile_type(url: &Url) -> Result<CompileType> {
    match url.scheme() {
        "file" | "" => {
            let path = url.to_file_path()
                .map_err(|()| "Could not get local file path for Url")?;

            if path.exists() && path.is_file() {
                let file_name = path.file_name().ok_or("Could not get file name")?;
                if file_name == "lib.toml" {
                    return Ok(CompileType::Library);
                }
                if file_name == "runner.toml" {
                    let runner_name = load_runner_spec(&path)?.name;
                    return Ok(CompileType::Runner(runner_name));
                }
            }

            if path.exists() && path.is_dir() {
                if path.join("lib.toml").exists() {
                    return Ok(CompileType::Library);
                }
                if path.join("runner.toml").exists() {
                    let spec_path =  path.join("runner.toml");
                    let runner_name = load_runner_spec(&spec_path)?.name;
                    return Ok(CompileType::Runner(runner_name));
                }
            }
        }
        _ => {}
    }

    Ok(CompileType::Flow)
}

/*
    run the loader to load the process and (optionally) compile, generate code and run the flow.
    Return either an error string if anything goes wrong or
    a message to display to the user if all went OK
*/
fn run() -> Result<()> {
    let options = parse_args(&get_matches())?;
    let mut lib_search_path = get_lib_search_path(&options.lib_dirs);

    let compile_type = compile_type(&options.source_url)?;

    match compile_type {
        CompileType::Library => {
            let output_dir = source_arg::get_output_dir(&options.source_url,
                                                        &options.output_dir,
                                                        compile_type)
                .chain_err(|| "Could not get the output directory")?;

            // Add the parent of the output_dir to the search path so compiler can find internal
            // references functions and flows during the lib build process
            let output_dir_parent = output_dir.parent()
                .ok_or("Could not get parent of output dir")?
                .to_string_lossy();
            lib_search_path.add(&output_dir_parent);
            let provider = &MetaProvider::new(lib_search_path,
                                              PathBuf::default());
            build_lib(&options, provider, &output_dir).chain_err(|| "Could not build library")
        },
        CompileType::Runner(_) => {
            let output_dir = source_arg::get_output_dir(&options.source_url,
                                                        &options.output_dir,
                                                        compile_type)
                .chain_err(|| "Could not get the output directory")?;
            build_runner(&options, &output_dir).chain_err(|| "Could not build runner")
        },
        CompileType::Flow => {
            let output_dir = source_arg::get_output_dir(&options.source_url,
                                                        &options.output_dir,
                                                        compile_type)
                .chain_err(|| "Could not get the output directory")?;

            let runner_name = options.runner_name.as_ref().ok_or("Runner name was not specified")?;
            let runner_dir = default_runner_dir(&runner_name.to_string());
            let provider = &MetaProvider::new(lib_search_path, runner_dir);
            compile_and_execute_flow(&options, provider, runner_name, &output_dir)
        }
    }
}

// Parse the command line arguments using clap
#[allow(clippy::too_many_lines)]
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
            Arg::new("runner")
                .short('r')
                .long("runner")
                .num_args(0..1)
                .number_of_values(1)
                .value_name("RUNNER_NAME")
                .help("The runner that will be used to run the flow"),
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
fn parse_args(matches: &ArgMatches) -> Result<Options> {
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
        .map_err(|()| "Could not form a Url for the current working directory")?;

    let source_url = url_from_string(&cwd_url,
                                     matches.get_one::<String>("source_url")
                                  .map(String::as_str))
        .chain_err(|| "Could not create a url for flow from the 'FLOW' command line parameter")?;

    let lib_dirs = if matches.contains_id("lib_dir") {
        matches
            .get_many::<String>("lib_dir")
            .chain_err(|| "Could not get the list of 'LIB_DIR' options specified")?
            .map(std::string::ToString::to_string)
            .collect()
    } else {
        vec![]
    };

    let flow_args = match matches.get_many::<String>("flow_args") {
        Some(strings) => strings.map(std::string::ToString::to_string).collect(),
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
        output_dir: matches.get_one::<String>("output").map(std::string::ToString::to_string),
        stdin_file: matches.get_one::<String>("stdin").map(std::string::ToString::to_string),
        lib_dirs,
        native_only: matches.get_flag("native"),
        runner_name: matches.get_one::<String>("runner").map(std::string::ToString::to_string),
        verbosity: verbosity_option.map(std::string::ToString::to_string),
        optimize: matches.get_flag("optimize")
    })
}

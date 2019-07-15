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

use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;

use clap::{App, AppSettings, Arg, ArgMatches};
use flowclib::compiler::compile;
use flowclib::compiler::loader;
use flowclib::dumper::dump_flow;
use flowclib::dumper::dump_tables;
use flowclib::generator::generate;
use flowclib::generator::generate::GenerationTables;
use flowclib::info;
use flowclib::model::flow::Flow;
use flowclib::model::process::Process::FlowProcess;
use flowrlib::manifest::DEFAULT_MANIFEST_FILENAME;
use simpath::FileType;
use simpath::Simpath;
use simplog::simplog::SimpleLogger;
use url::Url;

use provider::args::url_from_string;
use provider::content::provider::MetaProvider;

mod source_arg;

// We'll put our errors in an `errors` module, and other modules in
// this crate will `use errors::*;` to get access to everything
// `error_chain!` creates.
mod errors {
    // Create the Error, ErrorKind, ResultExt, and Result types
    error_chain! {}
}

error_chain! {
    foreign_links {
        Provider(::provider::errors::Error);
        Compiler(::flowclib::errors::Error);
        Io(::std::io::Error);
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
        Ok(message) => info!("{}", message)
    }
}

/*
    run the loader to load the process and (optionally) compile, generate code and run the flow.
    Return either an error string if anything goes wrong or
    a message to display to the user if all went OK
*/
fn run() -> Result<String> {
    let (url, args, dump, skip_generation, debug_symbols, out_dir)
        = parse_args(get_matches())?;
    let meta_provider = MetaProvider {};

    info!("==== Loader");
    match loader::load_context(&url.to_string(), &meta_provider)? {
        FlowProcess(flow) => {
            let tables = compile(&flow, dump, &out_dir).map_err(|e| e.to_string())?;

            if skip_generation {
                return Ok("Manifest generation and flow running skipped".to_string());
            }

            let manifest_path = write_manifest(flow, debug_symbols, out_dir, &tables).map_err(|e| e.to_string()).
                map_err(|e| e.to_string())?;

            // Append flow arguments at the end of the arguments so that they are passed on it when it's run
            execute_flow(manifest_path, args)
        }
        _ => bail!("Process loaded was not of type 'Flow' and cannot be executed")
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
            .help("Skip code generation and running"))
        .arg(Arg::with_name("dump")
            .short("d")
            .long("dump")
            .help("Dump the flow to standard output after loading it"))
        .arg(Arg::with_name("symbols")
            .short("g")
            .long("symbols")
            .help("Generate debug symbols (like process names and full routes)"))
        .arg(Arg::with_name("output")
            .short("o")
            .long("output")
            .takes_value(true)
            .value_name("OUTPUT_DIR")
            .help("Output directory for generated code"))
        .arg(Arg::with_name("log")
            .short("l")
            .long("log")
            .takes_value(true)
            .value_name("LOG_LEVEL")
            .help("Set log level for output (trace, debug, info, warn, error (default))"))
        .arg(Arg::with_name("FLOW")
            .help("the name of the 'flow' file")
            .required(false)
            .index(1))
        .arg(Arg::with_name("flow_args")
            .multiple(true))
        .get_matches()
}

/*
    Parse the command line arguments
*/
fn parse_args(matches: ArgMatches) -> Result<(Url, Vec<String>, bool, bool, bool, PathBuf)> {
    let mut args: Vec<String> = vec!();
    if let Some(flow_args) = matches.values_of("flow_args") {
        args = flow_args.map(|a| a.to_string()).collect();
    }

    SimpleLogger::init(matches.value_of("log"));

    info!("'{}' version {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
    info!("'flowclib' version {}\n", info::version());

    let url = url_from_string(matches.value_of("FLOW"))
        .chain_err(|| "Could not create a url for flow from the 'FLOW' command line paramter")?;

    let dump = matches.is_present("dump");
    let skip_generation = matches.is_present("skip");
    let debug_symbols = matches.is_present("symbols");
    let out_dir_option = matches.value_of("OUTPUT_DIR");
    let output_dir = source_arg::get_output_dir(&url, out_dir_option)
        .chain_err(|| "Could not get or create the output directory")?;

    Ok((url, args, dump, skip_generation, debug_symbols, output_dir))
}

fn compile(flow: &Flow, dump: bool, out_dir: &PathBuf) -> Result<GenerationTables> {
    info!("flow loaded with alias '{}'\n", flow.alias);

    let tables = compile::compile(&flow)?;

    if dump {
        dump_flow::dump_flow(&flow, &out_dir).map_err(|e| e.to_string())?;
        dump_tables::dump_tables(&tables, &out_dir).map_err(|e| e.to_string())?;
        dump_tables::dump_functions(&flow, &tables, &out_dir).map_err(|e| e.to_string())?;
    }

    Ok(tables)
}

fn write_manifest(flow: Flow, debug_symbols: bool, out_dir: PathBuf, tables: &GenerationTables)
                  -> Result<PathBuf> {
    let mut filename = out_dir.clone();
    filename.push(DEFAULT_MANIFEST_FILENAME.to_string());
    let mut manifest_file = File::create(&filename).chain_err(|| "Could not create manifest file")?;
    let out_dir_path = Url::from_file_path(out_dir).unwrap().to_string();

    let manifest = generate::create_manifest(&flow, debug_symbols, &out_dir_path, tables)
        .chain_err(|| "Could not create manifest from parsed flow and compiler tables")?;

    manifest_file.write_all(serde_json::to_string_pretty(&manifest)
        .chain_err(|| "Could not pretty format the manifest JSON contents")?
        .as_bytes()).chain_err(|| "Could not write manifest data bytes to created manifest file")?;

    Ok(filename)
}

#[cfg(not(target_os = "windows"))]
fn get_executable_name() -> String {
    "flowr".to_string()
}

#[cfg(target_os = "windows")]
fn get_executable_name() -> String {
    "flowr.exe".to_string()
}

/*
    Find the absolute path to the executable to be used to run the flow.
        - First looking for development directories under the Current Working Directory
          to facilitate development.
        - If not found there, then look in the PATH env variable
*/
fn find_executable_path(name: &str) -> Result<String> {
    // See if debug version in development is available
    let cwd = env::current_dir().map_err(|e| e.to_string())?;
    let file = cwd.join(&format!("./target/debug/{}", name));
    let abs_path = file.canonicalize();
    if let Ok(file_exists) = abs_path {
        return Ok(file_exists.to_string_lossy().to_string());
    }

    // Couldn't find the development version under CWD where running, so look in path
    let bin_search_path = Simpath::new("PATH");
    let bin_path = bin_search_path.find_type(name, FileType::File)
        .chain_err(|| format!("Could not find executable '{}'", name))?;

    Ok(bin_path.to_string_lossy().to_string())
}

/*
    Run flow using 'flowr'
    Inherit standard output and input and just let the process run as normal.
    Capture standard error.
    If the process exits correctly then just return an Ok() with message and no log
    If the process fails then return an Err() with message and log stderr in an ERROR level message
*/
fn execute_flow(filepath: PathBuf, mut args: Vec<String>) -> Result<String> {
    info!("==== Flowc: Executing flow from manifest in '{}'", filepath.display());

    let command = find_executable_path(&get_executable_name())?;
    let mut command_args = vec!(filepath.to_str().unwrap().to_string());
    command_args.append(&mut args);
    info!("Running flow using '{} {:?}'", &command, &command_args);
    let output = Command::new(&command).args(command_args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::piped())
        .output().map_err(|e| e.to_string())?;
    match output.status.code() {
        Some(0) => Ok("Flow ran to completion".to_string()),
        Some(code) => {
            error!("Process STDERR:\n{}", String::from_utf8_lossy(&output.stderr));
            bail!("Exited with status code: {}", code)
        }
        None => bail!("Process terminated by signal")
    }
}
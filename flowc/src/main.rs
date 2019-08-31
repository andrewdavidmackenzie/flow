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
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;

use clap::{App, AppSettings, Arg, ArgMatches};
use simpath::FileType;
use simpath::Simpath;
use simplog::simplog::SimpleLogger;
use tempdir::TempDir;
use url::Url;

use flowclib::compiler::compile;
use flowclib::compiler::loader;
use flowclib::dumper::dump_flow;
use flowclib::dumper::dump_tables;
use flowclib::generator::generate;
use flowclib::generator::generate::GenerationTables;
use flowclib::info;
use flowclib::model::flow::Flow;
use flowclib::model::function::Function;
use flowclib::model::process::Process::FlowProcess;
use flowrlib::manifest::DEFAULT_MANIFEST_FILENAME;
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
    let (url, args, dump, skip_generation, debug_symbols, provided_implementations, out_dir)
        = parse_args(get_matches())?;
    let meta_provider = MetaProvider {};

    info!("==== Loader");
    match loader::load_context(&url.to_string(), &meta_provider)? {
        FlowProcess(flow) => {
            let mut tables = compile(&flow, dump, &out_dir).chain_err(|| "Failed to compile")?;

            info!("==== Compiler phase: Compiling provided implementations");
            compile_supplied_implementations(&mut tables, provided_implementations)?;

            if skip_generation {
                return Ok("Manifest generation and flow running skipped".to_string());
            }

            info!("==== Compiler phase: Generating Manifest");
            let manifest_path = write_manifest(flow, debug_symbols, out_dir, &tables)
                .chain_err(|| "Failed to write manifest")?;

            // Append flow arguments at the end of the arguments so that they are passed on it when it's run
            info!("==== Compiler phase: Executing flow from manifest");
            execute_flow(manifest_path, args)
        }
        _ => bail!("Process loaded was not of type 'Flow' and cannot be executed")
    }
}

/*
    For any function that provides an implementation - compile the source to wasm and modify the
    implementation to indicate it is the wasm file
*/
fn compile_supplied_implementations(tables: &mut GenerationTables, skip_building: bool) -> Result<String> {
    for function in &mut tables.functions {
        match function.get_implementation() {
            Some(_) => compile_implementation(function, skip_building),
            None => Ok("OK".into())
        }?;
    }

    Ok("All supplied implementations compiled successfully".into())
}

/*
    Compile a function provided in rust to wasm and modify implementation to point to new file
*/
fn compile_implementation(function: &mut Box<Function>, skip_building: bool) -> Result<String> {
    let source = function.get_source_url();
    let mut implementation_url = url_from_string(Some(&source)).expect("Could not create a url from source url");
    implementation_url = implementation_url.join(&function.get_implementation()
        .ok_or("No implementation specified")?).map_err(|_| "Could not convert Url")?;

    // TODO what if not a file url? Copy and build locally?

    let implementation_path = implementation_url.to_file_path().expect("Could not convert source url to file path");
    if implementation_path.extension().ok_or("No file extension on source file")?.
        to_str().ok_or("Could not convert file extension to String")? != "rs" {
        bail!("Source file at '{}' does not have a '.rs' extension", implementation_path.display());
    }

    if !implementation_path.exists() {
        bail!("Source file at '{}' does not exist", implementation_path.display());
    }

    // check that a Cargo.toml file exists for compilation
    let mut cargo_path = implementation_path.clone();
    cargo_path.set_file_name("Cargo.toml");
    if !cargo_path.exists() {
        bail!("No Cargo.toml file could be found at '{}'", cargo_path.display());
    }
    info!("Building using rust manifest at '{}'", cargo_path.display());

    let mut wasm_destination = implementation_path.clone();
    wasm_destination.set_extension("wasm");

    // wasm file is out of date if it doesn't exist of timestamp is older than source
    let missing = !wasm_destination.exists();
    let out_of_date = missing || out_of_date(&implementation_path, &wasm_destination)?;

    if missing || out_of_date {
        if skip_building {
            if missing {
                let message = format!("Implementation at '{}' is missing so the flow cannot be executed.\nEither build manually or have 'flowc' build it by not using the '-p' option", wasm_destination.display());
                error!("{}", message);
                bail!(message);
            }
            if out_of_date {
                warn!("Implementation at '{}' is out of date with source at '{}'", wasm_destination.display(), implementation_path.display());
            }
        } else {
            info!("Building wasm '{}' from source '{}'", wasm_destination.display(), implementation_path.display());

            let build_dir = TempDir::new("flow")
                .expect("Error creating new TempDir for compiling in")
                .into_path();

            run_cargo_build(&cargo_path, &build_dir)?;

            // copy compiled wasm output into place where flow's toml file expects it
            let mut wasm_source = build_dir.clone();
            wasm_source.push("wasm32-unknown-unknown/release/");
            wasm_source.push(&wasm_destination.file_name().ok_or("Could not convert filename to str")?);
            info!("Copying built wasm from '{}' to '{}'", &wasm_source.display(), &wasm_destination.display());
            fs::copy(&wasm_source, &wasm_destination)?;

            // clean up temp dir
            fs::remove_dir_all(build_dir)?;
        }
    } else {
        info!("wasm at '{}' is up-to-date with source at '{}', so skipping build",
              wasm_destination.display(), implementation_path.display());
    }

    function.set_implementation(&wasm_destination.to_str().ok_or("Could not convert path to string")?);

    Ok("Function's provided implementation compiled successfully".into())
}

/*
    Run the cargo build to compile wasm from function source
*/
fn run_cargo_build(manifest_path: &PathBuf, target_dir: &PathBuf) -> Result<String> {
    info!("Building into temporary directory '{}'", target_dir.display());

    let command = "cargo";
    let mut command_args = vec!("build", "--quiet", "--release", "--lib", "--target=wasm32-unknown-unknown");
    let manifest = format!("--manifest-path={}", &manifest_path.display());
    command_args.push(&manifest);
    let target = format!("--target-dir={}", &target_dir.display());
    command_args.push(&target);

    debug!("Building with command = '{}', command_args = {:?}", command, command_args);

    let output = Command::new(&command).args(command_args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::piped())
        .output().chain_err(|| "Error while attempting to spawn command to compile and run flow")?;

    match output.status.code() {
        Some(0) => Ok("Cargo Build of supplied function to wasm succeeded".to_string()),
        Some(code) => {
            error!("Process STDERR:\n{}", String::from_utf8_lossy(&output.stderr));
            bail!("Exited with status code: {}", code)
        }
        None => Ok("No return code - ignoring".to_string())
    }
}

/*
    Determine if one file that is derived from another source is out of date (source is newer
    that derived)
    Returns:
        true - source is newer than derived
        false - source has not been modified since derived was created
*/
fn out_of_date(source: &PathBuf, derived: &PathBuf) -> Result<bool> {
    let source_last_modified = fs::metadata(source)?.modified()?;
    let derived_last_modified = fs::metadata(derived)?.modified()?;
    Ok(source_last_modified > derived_last_modified)
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
        .arg(Arg::with_name("log")
            .short("l")
            .long("log")
            .takes_value(true)
            .value_name("LOG_LEVEL")
            .help("Set log level for output (trace, debug, info, warn, error (default))"))
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
fn parse_args(matches: ArgMatches) -> Result<(Url, Vec<String>, bool, bool, bool, bool, PathBuf)> {
    let mut args: Vec<String> = vec!();
    if let Some(flow_args) = matches.values_of("flow_args") {
        args = flow_args.map(|a| a.to_string()).collect();
    }

    SimpleLogger::init(matches.value_of("log"));

    info!("'{}' version {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
    info!("'flowclib' version {}\n", info::version());

    let url = url_from_string(matches.value_of("FLOW"))
        .chain_err(|| "Could not create a url for flow from the 'FLOW' command line parameter")?;

    let dump = matches.is_present("dump");
    let skip_generation = matches.is_present("skip");
    let debug_symbols = matches.is_present("symbols");
    let provided_implementations = matches.is_present("provided");
    let out_dir_option = matches.value_of("OUTPUT_DIR");
    let output_dir = source_arg::get_output_dir(&url, out_dir_option)
        .chain_err(|| "Could not get or create the output directory")?;

    Ok((url, args, dump, skip_generation, debug_symbols, provided_implementations, output_dir))
}

fn compile(flow: &Flow, dump: bool, out_dir: &PathBuf) -> Result<GenerationTables> {
    info!("flow loaded with alias '{}'\n", flow.alias);

    let tables = compile::compile(&flow)?;

    if dump {
        dump_flow::dump_flow(&flow, &out_dir)
            .chain_err(|| "Failed to dump flow's definition")?;
        dump_tables::dump_tables(&tables, &out_dir)
            .chain_err(|| "Failed to dump flow's tables")?;
        dump_tables::dump_functions(&flow, &tables, &out_dir)
            .chain_err(|| "Failed to dump flow's functions")?;
    }

    Ok(tables)
}

/*
    Generate a manifest for the flow in JSON that can be used to run it using 'flowr'
*/
fn write_manifest(flow: Flow, debug_symbols: bool, out_dir_path: PathBuf, tables: &GenerationTables)
                  -> Result<PathBuf> {
    let mut filename = out_dir_path.clone();
    filename.push(DEFAULT_MANIFEST_FILENAME.to_string());
    let mut manifest_file = File::create(&filename).chain_err(|| "Could not create manifest file")?;
    let manifest = generate::create_manifest(&flow, debug_symbols, out_dir_path.to_str()
        .ok_or("Could not convert output directory to string")?, tables)
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
    let cwd = env::current_dir().chain_err(|| "Could not get the current working directory")?;
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
    info!("Executing flow from manifest in '{}'", filepath.display());

    let command = find_executable_path(&get_executable_name())?;
    let mut command_args = vec!(filepath.to_str().unwrap().to_string());
    command_args.append(&mut args);
    info!("Running flow using '{} {:?}'", &command, &command_args);
    let output = Command::new(&command).args(command_args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::piped())
        .output().chain_err(|| "Error while attempting to spawn command to compile and run flow")?;
    match output.status.code() {
        Some(0) => Ok("Flow ran to completion".to_string()),
        Some(code) => {
            error!("Process STDERR:\n{}", String::from_utf8_lossy(&output.stderr));
            bail!("Exited with status code: {}", code)
        }
        None => Ok("No return code - ignoring".to_string())
    }
}
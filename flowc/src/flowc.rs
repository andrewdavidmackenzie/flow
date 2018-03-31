#[macro_use]
extern crate log;
extern crate url;
extern crate tempdir;

extern crate clap;

use clap::{App, Arg, ArgMatches};

extern crate flowclib;

use flowclib::info;
use flowclib::loader::loader;
use flowclib::dumper::dumper;
use flowclib::compiler::compile;
use flowclib::generator::code_gen;
use std::process::Command;
use std::process::Stdio;
mod source_arg;

extern crate simplog;

use simplog::simplog::SimpleLogger;

fn main() {
    match run() {
        Ok(message) => info!("{}", message),
        Err(e) => error!("{}", e)
    }
}

/*
    Parse the command line arguments, then run the loader and (optional) compiling and code
    generation steps, returning either an error string if anything goes wrong along the way or
    a message to display to the user if all went OK
*/
fn run() -> Result<String, String> {
    let matches = get_matches();
    SimpleLogger::init(matches.value_of("log"));

    info!("'{}' version {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
    info!("'flowclib' version {}\n", info::version());

    let url = source_arg::url_from_cl_arg(matches.value_of("FLOW"))?;
    let dump = matches.is_present("dump");
    let skip_generation = matches.is_present("skip");

    info!("Attempting to load from url: '{}'", url);
    let mut flow = loader::load(&url)?;
    info!("'{}' flow loaded\n", flow.name);

    let tables = compile::compile(&mut flow)?;

    if dump {
        dumper::dump_flow(&flow);
        dumper::dump_tables(&tables);
    }

    if skip_generation {
        return Ok("Code Generation and Running skipped".to_string());
    }

    let output_dir = source_arg::get_output_dir(&flow.source_url,
                                                matches.value_of("OUTPUT_DIR"))?;

    let (build, run) =
        code_gen::generate(&flow, &output_dir, "Warn",
                           &tables, "rs").map_err(|e| e.to_string())?;

    info!("Building generated code in '{}' using '{}'", output_dir.display(), &build.0);
    build_flow(build)?;

    info!("Running generated code in '{}' using '{}'", output_dir.display(), &run.0);
    run_flow(run)
}

/*
    Run the build command, capturing all stdout and stderr.
    If everything executes fine, then return an Ok(), but don't produce any log or output
    If build fails, return an Err() with message and output the stderr in an ERROR level log message
*/
fn build_flow(command: (String, Vec<String>)) -> Result<String, String> {
    let build_output = Command::new(&command.0).args(command.1).output().map_err(|e| e.to_string())?;
    match build_output.status.code() {
        Some(0) => Ok(format!("'{}' command succeeded", command.0)),
        Some(code) => {
            error!("Build STDERR: \n {}", String::from_utf8_lossy(&build_output.stderr));
            return Err(format!("Exited with status code: {}", code));
        }
        None => {
            return Err("Terminated by signal".to_string());
        }
    }
}

/*
    Run flow that was previously built.
    Inherit standard output and input and just let the process run as normal.
    Capture standard error.
    If the process exits correctly then just return an Ok() with message and no log
    If the process fails then return an Err() with message and log stderr in an ERROR level message
*/
fn run_flow(command: (String, Vec<String>)) -> Result<String, String> {
    let output = Command::new(&command.0).args(command.1)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::piped())
        .output().map_err(|e| e.to_string())?;
    match output.status.code() {
        Some(0) => Ok("Flow ran to completion".to_string()),
        Some(code) => {
            error!("Process STDERR:\n{}", String::from_utf8_lossy(&output.stderr));
            Err(format!("Exited with status code: {}", code))
        }
        None => Err("Process terminated by signal".to_string())
    }
}

/*
    Parse the command line arguments using clap
*/
fn get_matches<'a>() -> ArgMatches<'a> {
    App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .arg(Arg::with_name("skip")
            .short("s")
            .long("skip")
            .help("Skip code generation and running"))
        .arg(Arg::with_name("dump")
            .short("d")
            .long("dump")
            .help("Dump the flow to standard output after loading it"))
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
        .get_matches()
}
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

mod source_arg;

extern crate simplog;

use simplog::simplog::SimpleLogger;

fn main() {
    match run() {
        Ok(message) => println!("{}\n", message),
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
    info!("'flowclib' version {}", info::version());

    let url = source_arg::url_from_cl_arg(matches.value_of("FLOW"))?;
    let dump = matches.is_present("dump");
    let generate = !matches.is_present("skip");

    info!("Attempting to load from url: '{}'", url);
    let mut flow = loader::load(&url)?;
    info!("'{}' flow loaded", flow.name);

    let tables = compile::compile(&mut flow);

    if dump {
        dumper::dump_flow(&flow);
        dumper::dump_tables(&tables);
    }

    if generate {
        let output_dir = source_arg::get_output_dir(&flow.source_url,
                                                    matches.value_of("OUTPUT_DIR"))?;

        let (command, args) = code_gen::generate(&flow, &output_dir, "Warn",
                                                 &tables)
            .map_err(|e| e.to_string())?;
        Command::new(&command).args(args).spawn().unwrap();

        Ok(format!("Executing generated code in '{}' using '{}'", output_dir.display(), &command))
    } else {
        Ok("Code Generation and Running skipped".to_string())
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
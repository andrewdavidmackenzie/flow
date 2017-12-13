#[macro_use]
extern crate log;
use log::LogLevelFilter;
use log::SetLoggerError;

extern crate glob;

extern crate clap;
use clap::{App, Arg};

extern crate flowclib;
use flowclib::info;
use flowclib::loader::loader;
use flowclib::compiler::compile;

mod files;
mod simple_logger;

use simple_logger::SimpleLogger;

use std::env;
use std::path::PathBuf;

fn init_logging() -> Result<(), SetLoggerError> {
    log::set_logger(|max_log_level| {
        max_log_level.set(LogLevelFilter::Info);
        Box::new(SimpleLogger)
    })
}

fn main() {
    init_logging().unwrap();

    info!("Logging started using 'log4rs', see log.yaml for configuration details");
    info!("'{}' version {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
    info!("'flowclib' version {}", info::version());

    let (path, dump, compile) = get_args();

    match files::get(path) {
        Ok(file_path) => {
            info!("Attempting to load file: '{}'", file_path.to_str().unwrap());
            match loader::load(file_path, dump) {
                Ok(mut flow) => {
                    info!("'{}' flow loaded", flow.name);

                    if compile {
                        compile::compile(&mut flow, dump);
                    }
                },
                Err(e) => error!("{}", e)
            }
        },
        Err(e) => error!("{}", e)
    }
}

fn get_args() -> (PathBuf, bool, bool) {
    let matches = App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .arg(Arg::with_name("load")
            .short("l")
            .help("Load the flow only, don't compile it"))
        .arg(Arg::with_name("dump")
            .short("d")
            .help("Dump the flow to standard output after loading it"))
        .arg(Arg::with_name("flow")
            .help("the name of the 'flow' file")
            .required(false)
            .index(1))
        .get_matches();

    // get the file name from the command line, use CDW if it is not present
    let path = match matches.value_of("flow") {
        None => {
            info!("No path specified, so using Current Working Directory");
            env::current_dir().unwrap()
        },
        Some(p) => PathBuf::from(p),
    };

    let dump = matches.is_present("dump");
    let compile = !matches.is_present("load");

    (path, dump, compile)
}
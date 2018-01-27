#[macro_use]
extern crate log;

use log::LogLevelFilter;

extern crate glob;

extern crate clap;

use clap::{App, Arg};

extern crate flowclib;
use flowclib::info;
use flowclib::loader::loader;
use flowclib::compiler::compile;

mod files;
mod file_arg;
mod simple_logger;

use simple_logger::SimpleLogger;

extern crate url;
use url::{Url, ParseError};

use std::env;

fn main() {
    init_logging();

    info!("Logging started using 'log4rs', see log.yaml for configuration details");
    info!("'{}' version {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
    info!("'flowclib' version {}", info::version());

    let (url, dump, compile) = get_args();

    match url {
        Ok(url) => {
            match files::find(url) {
                Ok(url) => {
                    info!("Attempting to load from url: '{}'", url);
                    match loader::load(&url, dump) {
                        Ok(mut flow) => {
                            info!("'{}' flow loaded", flow.name);

                            if compile {
                                compile::compile(&mut flow, dump);
                            }
                        }
                        Err(e) => error!("{}", e)
                    }
                }
                Err(e) => error!("{}", e)
            }
        },
        Err(e) => error!("{}", e)
    }

}

fn init_logging() {
    log::set_logger(|max_log_level| {
        max_log_level.set(LogLevelFilter::Info);
        Box::new(SimpleLogger)
    }).unwrap();
}

fn get_args() -> (Result<Url, ParseError>, bool, bool) {
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

    let parent = Url::from_directory_path(env::current_dir().unwrap()).unwrap();
    let url = file_arg::url_from_cl_arg(&parent, matches.value_of("flow"));
    let dump = matches.is_present("dump");
    let compile = !matches.is_present("load");

    (url, dump, compile)
}
extern crate glob;

extern crate clap;
use clap::{App, Arg};

extern crate flowlib;
use flowlib::info;
use flowlib::loader::loader;
use flowlib::dumper::dump;

mod files;

use std::env;
use std::path::PathBuf;

#[macro_use]
extern crate log;
extern crate log4rs;

fn main() {
    log4rs::init_file("log.yaml", Default::default()).unwrap();
    info!("Logging started using 'log4rs', see log.yaml for configuration details");
    info!("'{}' version {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
    info!("'flowlib' version {}", info::version());

    let matches = App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .arg(Arg::with_name("check")
            .short("c")
            .help("Check the flow only, don't execute it"))
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

    match files::get(path) {
        Ok(file_path) => {
            info!("Attempting to load file: '{:?}'", file_path);
            match loader::load_flow(file_path) {
                Ok(flow) => {
                    info!("'{}' flow parsed and validated correctly", flow.name);

                    if matches.is_present("dump") {
                        dump(flow, 0);
                    }

                    if !matches.is_present("check") {
                        // TODO run it
                    }
                },
                Err(e) => {
                    println!("{}", e);
                }
            }
        },
        Err(e) => println!("{}", e)
    }
}
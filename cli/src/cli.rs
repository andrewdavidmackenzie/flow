extern crate glob;

#[macro_use]
extern crate clap;
use clap::{App, Arg};

extern crate flowlib;
use flowlib::info;

mod commands;
mod files;

use std::env;

#[macro_use]
extern crate log;
extern crate log4rs;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

fn get_app<'a, 'b>() -> App<'a, 'b> {
    App::new("Flow CLI")
        .about("A Command Line tool for 'flow' programs")
        .author(crate_authors!())
        .version(crate_version!())
        .arg(Arg::with_name("verbose")
            .short("v")
            .long("verbose")
            .multiple(true)
            .help("Set the verbosity level - use multiple times to increase"))
}

fn main() {
    log4rs::init_file("log.yaml", Default::default()).unwrap();
    info!("Logging started using 'log4rs', see log.yaml for configuration details");
    info!("'flow' version {}", VERSION);
    info!("'flowlib' version {}", info::version());

    let app = get_app();
    commands::register(app); // app =

    // TODO Check if a file is specified - if not then look for the default one

    // TODO if no file is specified, then use the CWD
    let path = env::current_dir().unwrap();
    info!("No path specified, so using Current Working Directory: '{}'", path.display());

    // Try and find the default file by passing a directory
    match files::open(path) {
        Ok(file) => commands::validate::validate(file),
        Err(_) => {
            println!("No file found");
        }
    }

    // TODO pass the file to the command parsed by clap
}
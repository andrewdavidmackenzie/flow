#[macro_use]
extern crate log;
extern crate url;
extern crate tempdir;

use log::LogLevelFilter;

extern crate clap;

use clap::{App, Arg, ArgMatches};

extern crate flowclib;

use flowclib::info;
use flowclib::loader::loader;
use flowclib::dumper::dumper;
use flowclib::content::provider;
use flowclib::compiler::compile;

mod source_arg;
mod simple_logger;

use simple_logger::SimpleLogger;

fn main() {
    init_logging();

    info!("'{}' version {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
    info!("'flowclib' version {}", info::version());

    match run() {
        Ok(_) => {}
        Err(e) => error!("{}", e)
    }
}

fn init_logging() {
    log::set_logger(|max_log_level| {
        max_log_level.set(LogLevelFilter::Info);
        Box::new(SimpleLogger)
    }).unwrap();
    info!("Logging started using 'log4rs', see log.yaml for configuration details");
}

/*
    Parse the command line arguments, then run the loader and (optional) compiling steps,
    returning early (with an error string) if anything goes wrong along the way.
*/
fn run() -> Result<(), String> {
    let matches = get_matches();
    let mut url = source_arg::url_from_cl_arg(matches.value_of("FLOW"))?;
    let dump = matches.is_present("dump");
    let compile = !matches.is_present("load");

    // The specified url maybe a directory or a specific file, see if we can find the flow to load
    info!("Attempting to find flow using url: '{}'", url);
    url = provider::find(&url)?;

    info!("Attempting to load from url: '{}'", url);
    let mut flow = loader::load(&url)?;
    info!("'{}' flow loaded", flow.name);

    if dump {
        dumper::dump(&flow);
    }

    if compile {
        let output_dir = source_arg::get_output_dir(&url, matches.value_of("OUTPUT_DIR"));
        info!("Generating rust project into dir '{}'", output_dir.to_str().unwrap());
        compile::compile(&mut flow, &output_dir, dump)
    } else {
        info!("Compiling skipped");
        Ok(())
    }
}

/*
    Parse the command line arguments using clap
*/
fn get_matches<'a>() -> ArgMatches<'a> {
    App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .arg(Arg::with_name("load")
            .short("l")
            .help("Load the flow only, don't compile it"))
        .arg(Arg::with_name("dump")
            .short("d")
            .help("Dump the flow to standard output after loading it"))
        .arg(Arg::with_name("output")
            .short("o")
            .takes_value(true)
            .value_name("OUTPUT_DIR")
            .help("Output directory for generated code"))
        .arg(Arg::with_name("FLOW")
            .help("the name of the 'flow' file")
            .required(false)
            .index(1))
        .get_matches()
}
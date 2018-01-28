#[macro_use]
extern crate log;

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

extern crate url;

use url::Url;

use std::env;

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
    let mut url = get_url(&matches)?;
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
        compile::compile(&mut flow, dump)
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
        .arg(Arg::with_name("flow")
            .help("the name of the 'flow' file")
            .required(false)
            .index(1))
        .get_matches()
}

/*
    Use the current working directory as the starting point ("parent") for parsing a command
    line specified url where to load the flow from. This allows specifiying of full Urls
    (http, file etc) as well as file paths relative to the working directory.

    Returns a full url with appropriate scheme, and an absolute path.
*/
fn get_url(matches: &ArgMatches) -> Result<Url, String> {
    let parent = Url::from_directory_path(env::current_dir().unwrap()).unwrap();
    source_arg::url_from_cl_arg(&parent, matches.value_of("flow"))
}
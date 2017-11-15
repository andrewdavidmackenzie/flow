use clap::{App, SubCommand};
use std::fs::File;

pub fn register<'a, 'b>(app: App<'a, 'b>) -> App<'a, 'b> {
    let subcommand = configure_subcommand(SubCommand::with_name("validate"));
    app.subcommand(subcommand)
}

fn configure_subcommand<'a, 'b>(cmd: App<'a, 'b>) -> App<'a, 'b> {
    cmd.about("Validate a flow")
}

pub fn validate(file: File) {
    info!("Validating file: '{:?}'", file);

    /* TODO re enable this when can get lib to compile
    match parser::load(&path, true) {
        parser::Result::ContextLoaded(context) => info!("'{}' context parsed and validated correctly", context.name),
        parser::Result::FlowLoaded(flow) => info!("'{}' flow parsed and validated correctly", flow.name),
        parser::Result::Error(error) => error!("{}", error),
        parser::Result::Valid => error!("Unexpected parser failure"),
    }
    */


    /*

    validate model (see check)

    load flow definition from file specified in arguments
        - load any referenced to included flows also

    construct overall list of functions

    construct list of connections

    construct initial list of all functions able to produce output
        - start from external sources at level 0

    do
        - identify all functions which receive input from active sources
        - execute all those functions
        - functions producing output added to list of active sources
    while functions pending input

     */

}
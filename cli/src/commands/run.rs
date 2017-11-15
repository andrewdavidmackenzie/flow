use clap::{App, SubCommand};

pub fn register<'a, 'b>(app: App<'a, 'b>) -> App<'a, 'b> {
    let subcommand = configure_subcommand(SubCommand::with_name("run"));
    app.subcommand(subcommand)
}

fn configure_subcommand<'a, 'b>(cmd: App<'a, 'b>) -> App<'a, 'b> {
    cmd.about("Run a flow")
}

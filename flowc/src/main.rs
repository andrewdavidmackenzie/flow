extern crate clap;
extern crate flowclib;
extern crate flowrlib;
#[macro_use]
extern crate log;
extern crate provider;
extern crate serde_json;
extern crate simplog;
extern crate tempdir;
extern crate url;

use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;

use clap::{App, AppSettings, Arg, ArgMatches};
use flowclib::compiler::compile;
use flowclib::dumper::dump_flow;
use flowclib::dumper::dump_tables;
use flowclib::generator::generate;
use flowclib::generator::generate::GenerationTables;
use flowclib::info;
use flowclib::loader::loader;
use flowclib::model::flow::Flow;
use flowclib::model::process::Process::FlowProcess;
use simplog::simplog::SimpleLogger;
use url::Url;

use provider::content::args::url_from_string;
use provider::content::provider::MetaProvider;

mod source_arg;

fn main() {
    match run() {
        Ok(message) => info!("{}", message),
        Err(e) => error!("{}", e)
    }
}

/*
    run the loader to load the process and (optionally) compile, generate code and run the flow.
    Return either an error string if anything goes wrong or
    a message to display to the user if all went OK
*/
fn run() -> Result<String, String> {
    let (url, args, dump, skip_generation, out_dir) = parse_args(get_matches())?;
    let meta_provider = MetaProvider {};

    let process = loader::load_process(&"".to_string(),
                                       &"context".to_string(), &url.to_string(), &meta_provider)?;
    match process {
        FlowProcess(flow) => run_flow(flow, args, dump, skip_generation, out_dir),
        _ => Err(format!("Process loaded was not of type 'Flow' and cannot be executed"))
    }
}

/*
    Parse the command line arguments using clap
*/
fn get_matches<'a>() -> ArgMatches<'a> {
    App::new(env!("CARGO_PKG_NAME"))
        .setting(AppSettings::TrailingVarArg)
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
        .arg(Arg::with_name("flow_args")
            .multiple(true))
        .get_matches()
}

/*
    Parse the command line arguments
*/
fn parse_args(matches: ArgMatches) -> Result<(Url, Vec<String>, bool, bool, PathBuf), String> {
    let mut args: Vec<String> = vec!();
    if let Some(flow_args) = matches.values_of("flow_args") {
        args = flow_args.map(|a| a.to_string()).collect();
    }

    SimpleLogger::init(matches.value_of("log"));

    info!("'{}' version {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
    info!("'flowclib' version {}\n", info::version());

    let url = url_from_string(matches.value_of("FLOW"))?;

    let dump = matches.is_present("dump");
    let skip_generation = matches.is_present("skip");
    let out_dir_option = matches.value_of("OUTPUT_DIR");
    let output_dir = source_arg::get_output_dir(&url, out_dir_option)?;

    Ok((url, args, dump, skip_generation, output_dir))
}

fn run_flow(flow: Flow, args: Vec<String>, dump: bool, skip_generation: bool, out_dir: PathBuf)
            -> Result<String, String> {
    info!("flow loaded with alias '{}'\n", flow.alias);

    let tables = compile::compile(&flow)?;

    if dump {
        info!("Dumping flow, compiler tables and runnable descriptions in '{}'", out_dir.display());
        dump_flow::dump_flow(&flow, &out_dir).map_err(|e| e.to_string())?;
        dump_tables::dump_tables(&tables, &out_dir).map_err(|e| e.to_string())?;
        dump_tables::dump_runnables(&flow, &tables, &out_dir).map_err(|e| e.to_string())?;
    }

    if skip_generation {
        return Ok("Manifest generation and flow running skipped".to_string());
    }

    let manifest_path= write_manifest(&flow, &out_dir, &tables).map_err(|e| e.to_string())?;

    // Append flow arguments at the end of the arguments so that they are passed on it when it's run
    execute_flow(manifest_path, args)
}

/*
    let dir = TempDir::new("flow")
        .map_err(|e| format!("Error creating new TempDir, \n'{}'", e.to_string()))?;
    let mut output_dir = dir.into_path();
*/

fn write_manifest(flow: &Flow, out_dir: &PathBuf, tables: &GenerationTables)
                  -> Result<PathBuf, std::io::Error> {
    let mut filename = out_dir.clone();
    filename.push("manifest.json".to_string());
    let mut manifest_file = File::create(&filename)?;
    let out_dir_path = Url::from_file_path(out_dir).unwrap().to_string();

    let manifest = generate::create_manifest(flow, &out_dir_path, tables)?;

    manifest_file.write_all(serde_json::to_string_pretty(&manifest)?.as_bytes())?;

    Ok(filename)
}

/*
    Run flow using 'flowr'
    Inherit standard output and input and just let the process run as normal.
    Capture standard error.
    If the process exits correctly then just return an Ok() with message and no log
    If the process fails then return an Err() with message and log stderr in an ERROR level message
*/
fn execute_flow(filepath: PathBuf, mut args: Vec<String>) -> Result<String, String> {
    let command = "cargo".to_string();
    let mut command_args = vec!("run".to_string(), "--bin".to_string(), "flowr".to_string());
    command_args.push(filepath.to_str().unwrap().to_string());
    command_args.append(&mut args);
    info!("Running flow using '{} {:?}'", &command, &command_args);
    let output = Command::new(&command).args(command_args)
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

#[cfg(test)]
mod test {
    extern crate flowclib;
    extern crate url;
    extern crate provider;

    use std::env;

    use flowclib::compiler::compile;
    use flowclib::loader::loader;
    use flowclib::model::name::Name;
    use flowclib::model::process::Process::FlowProcess;
    use url::Url;

    use provider::content::args::url_from_string;
    use provider::content::provider::MetaProvider;

    fn url_from_rel_path(path: &str) -> String {
        let cwd = Url::from_file_path(env::current_dir().unwrap()).unwrap();
        cwd.join(path).unwrap().to_string()
    }

    #[test]
    fn compile_args_ok() {
        let meta_provider = MetaProvider {};
        let parent_route = &"".to_string();
        let path = url_from_rel_path("flowc/test-flows/args.toml");
        let process = loader::load_process(parent_route, &"context".to_string(),
                                           &path, &meta_provider).unwrap();
        if let FlowProcess(ref flow) = process {
            let _tables = compile::compile(flow).unwrap();
        } else {
            assert!(false, "Process loaded was not a flow");
        }
    }

    #[test]
    fn compile_echo_ok() {
        let meta_provider = MetaProvider {};
        let parent_route = &"".to_string();
        let process = loader::load_process(parent_route, &"echo".to_string(),
                                           &url_from_rel_path("flowc/test-flows/echo.toml"),
                                           &meta_provider).unwrap();
        if let FlowProcess(ref flow) = process {
            let _tables = compile::compile(flow).unwrap();
        } else {
            assert!(false, "Process loaded was not a flow");
        }
    }

    #[test]
    #[should_panic]
    fn compiler_detects_competing_inputs() {
        let meta_provider = MetaProvider {};
        let parent_route = &"".to_string();
        let process = loader::load_process(parent_route, &"competing".to_string(),
                                           &url_from_rel_path("flowc/test-flows/competing.toml"),
                                           &meta_provider).unwrap();
        if let FlowProcess(ref flow) = process {
            let _tables = compile::compile(flow).unwrap();
        } else {
            assert!(false, "Process loaded was not a flow");
        }
    }

    #[test]
    #[should_panic]
    fn compiler_detects_unused_input() {
        let meta_provider = MetaProvider {};
        let parent_route = &"".to_string();
        let process = loader::load_process(parent_route, &"unused_input".to_string(),
                                           &url_from_rel_path("flowc/test-flows/unused_input.toml"),
                                           &meta_provider).unwrap();
        if let FlowProcess(ref flow) = process {
            let _tables = compile::compile(flow).unwrap();
        } else {
            assert!(false, "Process loaded was not a flow");
        }
    }

    #[test]
    #[should_panic]
    fn compiler_detects_loop() {
        let meta_provider = MetaProvider {};
        let parent_route = &"".to_string();
        let process = loader::load_process(parent_route, &"loop".to_string(),
                                           &url_from_rel_path("flowc/test-flows/loop.toml"),
                                           &meta_provider).unwrap();
        if let FlowProcess(ref flow) = process {
            let _tables = compile::compile(flow).unwrap();
        } else {
            assert!(false, "Process loaded was not a flow");
        }
    }

    #[test]
    #[should_panic]
    fn compile_double_connection() {
        let meta_provider = MetaProvider {};
        let parent_route = &"".to_string();
        let process = loader::load_process(parent_route, &Name::from("double"),
                                           &url_from_rel_path("flowc/test-flows/double.toml"),
                                           &meta_provider).unwrap();
        if let FlowProcess(ref flow) = process {
            let _tables = compile::compile(flow).unwrap();
        } else {
            assert!(false, "Process loaded was not a flow");
        }
    }

    #[test]
    fn load_hello_world_simple_from_context() {
        let meta_provider = MetaProvider {};
        let parent_route = &"".to_string();
        loader::load_process(parent_route, &"hello-world-simple".to_string(),
                             &url_from_rel_path("samples/hello-world-simple/context.toml"),
                             &meta_provider).unwrap();
    }

    #[test]
    fn load_hello_world_from_context() {
        let meta_provider = MetaProvider {};
        let parent_route = &"".to_string();
        loader::load_process(parent_route, &"hello-world".to_string(),
                             &url_from_rel_path("samples/hello-world/context.toml"),
                             &meta_provider).unwrap();
    }

    #[test]
    fn load_hello_world_include() {
        let meta_provider = MetaProvider {};
        let parent_route = &"".to_string();
        loader::load_process(parent_route, &"hello-world-include".to_string(),
                             &url_from_rel_path("samples/hello-world-include/context.toml"),
                             &meta_provider).unwrap();
    }

    #[test]
    fn load_hello_world_flow1() {
        let meta_provider = MetaProvider {};
        let parent_route = &"".to_string();
        loader::load_process(parent_route, &"flow1".to_string(),
                             &url_from_rel_path("samples/hello-world/flow1.toml"),
                             &meta_provider).unwrap();
    }

    #[test]
    #[ignore]
    fn load_reverse_echo_from_toml() {
        let meta_provider = MetaProvider {};
        let parent_route = &"".to_string();
        loader::load_process(parent_route, &"reverse-echo".to_string(),
                             &url_from_rel_path("samples/reverse-echo/context.toml"),
                             &meta_provider).unwrap();
    }

    #[test]
    fn load_fibonacci_from_toml() {
        let meta_provider = MetaProvider {};
        let parent_route = &"".to_string();
        loader::load_process(parent_route, &"fibonacci".to_string(),
                             &url_from_rel_path("samples/fibonacci/context.toml"),
                             &meta_provider).unwrap();
    }

    #[test]
    fn load_fibonacci_from_directory() {
        let meta_provider = MetaProvider {};
        let parent_route = &"".to_string();
        let url = url_from_string(Some("../samples/fibonacci")).unwrap();
        loader::load_process(parent_route, &"fibonacci".to_string(),
                             &url.into_string(), &meta_provider).unwrap();
    }
}
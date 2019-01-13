extern crate clap;
extern crate curl;
extern crate flowclib;
extern crate glob;
#[macro_use]
extern crate log;
extern crate simpath;
extern crate simplog;
extern crate tempdir;
extern crate url;

use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;

use clap::{App, AppSettings, Arg, ArgMatches};
use flowclib::compiler::compile;
use flowclib::dumper::dump_flow;
use flowclib::dumper::dump_tables;
use flowclib::generator::code_gen;
use flowclib::info;
use flowclib::loader::loader;
use flowclib::model::flow::Flow;
use flowclib::model::process::Process::FlowProcess;
use simplog::simplog::SimpleLogger;
use url::Url;

mod source_arg;
mod content;

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
    let (url, args, dump, skip_generation, out_dir) = parse_args( get_matches())?;
    let meta_provider = content::provider::MetaProvider {};

    let process = loader::load_process(&"".to_string(),
                                        &"context".to_string(), &url, &meta_provider)?;
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

    let url = source_arg::url_from_cl_arg(matches.value_of("FLOW"))?;

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
        // Dump data describing flows and tables in the parent directory of the code generation
        let mut dump_dir = out_dir.clone();
        dump_dir.pop();
        info!("Dumping flow, compiler tables and runnable descriptions in '{}'", dump_dir.display());

        dump_flow::dump_flow(&flow, &dump_dir).map_err(|e| e.to_string())?;
        dump_tables::dump_tables(&tables, &dump_dir).map_err(|e| e.to_string())?;
        dump_tables::dump_runnables(&flow, &tables, &dump_dir).map_err(|e| e.to_string())?;
    }

    if skip_generation {
        return Ok("Code Generation and Running skipped".to_string());
    }

    let (build, run) =
        code_gen::generate(&flow, &out_dir, "Warn",
                           &tables, "rs").map_err(|e| e.to_string())?;

    build_flow(build)?;

    // Append flow arguments at the end of the arguments so that are passed on it when it's run
    execute_flow(run, args)
}
/*
    Run the build command, capturing all stdout and stderr.
    If everything executes fine, then return an Ok(), but don't produce any log or output
    If build fails, return an Err() with message and output the stderr in an ERROR level log message
*/
fn build_flow(command: (String, Vec<String>)) -> Result<String, String> {
    info!("Building generated code using '{} {:?}'", &command.0, &command.1);

    let build_output = Command::new(&command.0).args(command.1).output().map_err(|e| e.to_string())?;
    match build_output.status.code() {
        Some(0) => Ok(format!("'{}' command succeeded", command.0)),
        Some(code) => {
            error!("Build STDERR: \n {}", String::from_utf8_lossy(&build_output.stderr));
            return Err(format!("Exited with status code: {}", code));
        }
        None => {
            return Err("Terminated by signal".to_string());
        }
    }
}

/*
    Run flow that was previously built.
    Inherit standard output and input and just let the process run as normal.
    Capture standard error.
    If the process exits correctly then just return an Ok() with message and no log
    If the process fails then return an Err() with message and log stderr in an ERROR level message
*/
fn execute_flow(command: (String, Vec<String>), mut flow_args: Vec<String>) -> Result<String, String> {
    let mut run_args = command.1.clone();
    run_args.append(&mut flow_args);
    info!("Running generated code using '{} {:?}'", &command.0, &run_args);
    let output = Command::new(&command.0).args(run_args)
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

    use std::env;

    use flowclib::compiler::compile;
    use flowclib::loader::loader;
    use flowclib::model::name::Name;
    use flowclib::model::process::Process::FlowProcess;
    use url::Url;

    use content::provider::MetaProvider;
    use source_arg::url_from_cl_arg;

    fn url_from_rel_path(path: &str) -> Url {
        let cwd = Url::from_file_path(env::current_dir().unwrap()).unwrap();
        cwd.join(path).unwrap()
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
    fn compiled_detects_competing_inputs() {
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
        let url = url_from_cl_arg(Some("../samples/fibonacci")).unwrap();
        println!("url = {}", url);
        loader::load_process(parent_route, &"fibonacci".to_string(),
                     &url, &meta_provider).unwrap();
    }
}
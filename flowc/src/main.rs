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
use flowclib::compiler::loader;
use flowclib::dumper::dump_flow;
use flowclib::dumper::dump_tables;
use flowclib::generator::generate;
use flowclib::generator::generate::GenerationTables;
use flowclib::info;
use flowclib::model::flow::Flow;
use flowclib::model::process::Process::FlowProcess;
use flowrlib::manifest::DEFAULT_MANIFEST_FILENAME;
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
    let (url, args, dump, skip_generation, debug_symbols, out_dir)
        = parse_args(get_matches())?;
    let meta_provider = MetaProvider {};

    info!("==== Loader");
    let process = loader::load_context(&url.to_string(), &meta_provider)?;
    match process {
        FlowProcess(flow) => compile_and_execute(flow, args, dump, skip_generation, debug_symbols, out_dir),
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
        .arg(Arg::with_name("symbols")
            .short("g")
            .long("symbols")
            .help("Generate debug symbols (like process names and full routes)"))
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
fn parse_args(matches: ArgMatches) -> Result<(Url, Vec<String>, bool, bool, bool, PathBuf), String> {
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
    let debug_symbols = matches.is_present("symbols");
    let out_dir_option = matches.value_of("OUTPUT_DIR");
    let output_dir = source_arg::get_output_dir(&url, out_dir_option)?;

    Ok((url, args, dump, skip_generation, debug_symbols, output_dir))
}

fn compile_and_execute(flow: Flow, args: Vec<String>, dump: bool, skip_generation: bool, debug_symbols: bool, out_dir: PathBuf)
                       -> Result<String, String> {
    info!("flow loaded with alias '{}'\n", flow.alias);

    let tables = compile::compile(&flow)?;

    if dump {
        dump_flow::dump_flow(&flow, &out_dir).map_err(|e| e.to_string())?;
        dump_tables::dump_tables(&tables, &out_dir).map_err(|e| e.to_string())?;
        dump_tables::dump_runnables(&flow, &tables, &out_dir).map_err(|e| e.to_string())?;
    }

    if skip_generation {
        return Ok("Manifest generation and flow running skipped".to_string());
    }

    let manifest_path = write_manifest(&flow, debug_symbols, &out_dir, &tables).map_err(|e| e.to_string())?;

    // Append flow arguments at the end of the arguments so that they are passed on it when it's run
    execute_flow(manifest_path, args)
}

/*
    let dir = TempDir::new("flow")
        .map_err(|e| format!("Error creating new TempDir, \n'{}'", e.to_string()))?;
    let mut output_dir = dir.into_path();
*/

fn write_manifest(flow: &Flow, debug_symbols: bool, out_dir: &PathBuf, tables: &GenerationTables)
                  -> Result<PathBuf, std::io::Error> {
    let mut filename = out_dir.clone();
    filename.push(DEFAULT_MANIFEST_FILENAME.to_string());
    let mut manifest_file = File::create(&filename)?;
    let out_dir_path = Url::from_file_path(out_dir).unwrap().to_string();

    let manifest = generate::create_manifest(flow, debug_symbols, &out_dir_path, tables)?;

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
    info!("==== Flowc: Executing flow from manifest in '{}'", filepath.display());

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
    use flowclib::compiler::loader;
    use flowclib::model::io::IO;
    use flowclib::model::name::HasName;
    use flowclib::model::process::Process::FlowProcess;
    use flowclib::model::process::Process::FunctionProcess;
    use flowclib::model::route::HasRoute;
    use flowrlib::input::InputInitializer::OneTime;
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
        let path = url_from_rel_path("flowc/test-flows/args.toml");
        let process = loader::load_context(&path, &meta_provider).unwrap();
        if let FlowProcess(ref flow) = process {
            let _tables = compile::compile(flow).unwrap();
        } else {
            assert!(false, "Process loaded was not a flow");
        }
    }

    #[test]
    fn dead_process_removed() {
        let meta_provider = MetaProvider {};
        let path = url_from_rel_path("flowc/test-flows/dead-process.toml");
        let process = loader::load_context(&path, &meta_provider).unwrap();
        if let FlowProcess(ref flow) = process {
            let tables = compile::compile(flow).unwrap();
            // Dead value should be removed - currently can't assume that args function can be removed
            assert_eq!(tables.functions.len(), 1, "Incorrect number of runnables after optimization");
            assert_eq!(tables.functions.get(0).unwrap().get_id(), 0,
                       "Runnables indexes do not start at 0");
            // And the connection to it also
            assert_eq!(tables.collapsed_connections.len(), 0, "Incorrect number of connections after optimization");
        } else {
            assert!(false, "Process loaded was not a flow");
        }
    }

    #[test]
    fn dead_process_and_connected_process_removed() {
        let meta_provider = MetaProvider {};
        let path = url_from_rel_path("flowc/test-flows/dead-process-and-connected-process.toml");
        let process = loader::load_context(&path, &meta_provider).unwrap();
        if let FlowProcess(ref flow) = process {
            let tables = compile::compile(flow).unwrap();
            assert!(tables.functions.is_empty(), "Incorrect number of runnables after optimization");
            // And the connection are all gone also
            assert_eq!(tables.collapsed_connections.len(), 0, "Incorrect number of connections after optimization");
        } else {
            assert!(false, "Process loaded was not a flow");
        }
    }

    #[test]
    fn compile_echo_ok() {
        let meta_provider = MetaProvider {};
        let process = loader::load_context(&url_from_rel_path("flowc/test-flows/echo.toml"),
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
        let process = loader::load_context(&url_from_rel_path("flowc/test-flows/unused_input.toml"),
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
        let process = loader::load_context(&url_from_rel_path("flowc/test-flows/loop.toml"),
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
        let process = loader::load_context(&url_from_rel_path("flowc/test-flows/double.toml"),
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
        loader::load_context(&url_from_rel_path("samples/hello-world-simple/context.toml"),
                             &meta_provider).unwrap();
    }

    #[test]
    fn load_hello_world_from_context() {
        let meta_provider = MetaProvider {};
        loader::load_context(&url_from_rel_path("samples/hello-world/context.toml"),
                             &meta_provider).unwrap();
    }

    #[test]
    fn load_hello_world_include() {
        let meta_provider = MetaProvider {};
        loader::load_context(&url_from_rel_path("samples/hello-world-include/context.toml"),
                             &meta_provider).unwrap();
    }

    #[test]
    fn load_hello_world_flow1() {
        let meta_provider = MetaProvider {};
        loader::load_context(&url_from_rel_path("samples/hello-world/flow1.toml"),
                             &meta_provider).unwrap();
    }

    #[test]
    fn load_reverse_echo_from_toml() {
        let meta_provider = MetaProvider {};
        loader::load_context(&url_from_rel_path("samples/reverse-echo/context.toml"),
                             &meta_provider).unwrap();
    }

    #[test]
    fn load_fibonacci_from_toml() {
        let meta_provider = MetaProvider {};
        loader::load_context(&url_from_rel_path("samples/fibonacci/context.toml"),
                             &meta_provider).unwrap();
    }

    #[test]
    fn load_fibonacci_from_directory() {
        let meta_provider = MetaProvider {};
        let url = url_from_string(Some("../samples/fibonacci")).unwrap();
        loader::load_context(&url.into_string(), &meta_provider).unwrap();
    }

    #[test]
    fn function_input_initialized() {
        let meta_provider = MetaProvider {};
        let url = url_from_rel_path("flowc/test-flows/function_input_init.toml");

        match loader::load_context(&url, &meta_provider) {
            Ok(FlowProcess(flow)) => {
                if let FunctionProcess(ref print_function) = flow.process_refs.unwrap()[0].process {
                    assert_eq!(print_function.alias(), "print", "Function alias does not match");
                    if let Some(inputs) = print_function.get_inputs() {
                        let default_input: &IO = inputs.get(0).unwrap();
                        let initial_value = default_input.get_initial_value().clone().unwrap();
                        match initial_value {
                            OneTime(one_time) => assert_eq!(one_time.once, "hello"),
                            _ => panic!("Initializer should have been a OneTime initializer")
                        }
                    } else {
                        panic!("Could not find any inputs");
                    }
                } else {
                    panic!("First sub-process was not a function as expected")
                }
            }
            Ok(_) => panic!("Didn't load a flow"),
            Err(e) => panic!(e.to_string())
        }
    }

    /*
        This tests that an initalizer on an input to a flow process is passed onto function processes
        inside the flow, via a connection from the flow input to the function input
    */
    #[test]
    fn flow_input_initialized_and_propogated_to_function() {
        let meta_provider = MetaProvider {};
        // Relative path from project root to the test file
        let url = url_from_rel_path("flowc/test-flows/flow_input_init.toml");

        match loader::load_context(&url, &meta_provider) {
            Ok(FlowProcess(flow)) => {
                if let FlowProcess(ref pilte_sub_flow) = flow.process_refs.unwrap()[0].process {
                    assert_eq!("pass-if-lte", pilte_sub_flow.alias(), "Flow alias is not 'pass-if-lte' as expected");

                    if let Some(ref process_refs) = pilte_sub_flow.process_refs {
                        if let FunctionProcess(ref tap_function) = process_refs.get(0).unwrap().process {
                            assert_eq!("tap", tap_function.alias(), "Function alias is not 'tap' as expected");
                            if let Some(inputs) = tap_function.get_inputs() {
                                let in_input = inputs.get(0).unwrap();
                                assert_eq!("data", in_input.alias(), "Input's name is not 'data' as expected");
                                assert_eq!("/context/pass-if-lte/tap/data", in_input.route(), "Input's route is not as expected");
                                let initial_value = in_input.get_initial_value();
                                match initial_value {
                                    Some(OneTime(one_time)) => assert_eq!(one_time.once, 1),
                                    _ => panic!("Initializer should have been a OneTime initializer")
                                }
                            } else {
                                panic!("Could not find any inputs");
                            }
                        } else {
                            panic!("First sub-process of 'pass-if-lte' sub-flow was not a function as expected");
                        }
                    } else {
                        panic!("Could not get process_refs of sub_flow");
                    }
                } else {
                    panic!("First sub-process of context flow was not a sub-flow as expected")
                }
            }
            Ok(_) => panic!("Didn't load a flow"),
            Err(e) => panic!(e.to_string())
        }
    }


    /*
        This tests that an initalizer on an input to a flow process is passed onto a function in
        a sub-flow of that via a connection from the flow input to the function input
    */
    #[test]
    #[should_panic]
    fn flow_input_initialized_and_propogated_to_function_in_subflow() {
        let meta_provider = MetaProvider {};
        // Relative path from project root to the test file
        let url = url_from_rel_path("flowc/test-flows/subflow_function_input_init.toml");

        match loader::load_context(&url, &meta_provider) {
            Ok(FlowProcess(context)) => {
                if let FlowProcess(ref sequence_sub_flow) = context.process_refs.unwrap()[0].process {
                    assert_eq!("sequence", sequence_sub_flow.alias(), "First sub-flow alias is not 'sequence' as expected");

                    if let Some(ref sequence_process_refs) = sequence_sub_flow.process_refs {
                        if let FlowProcess(ref pilte_sub_flow) = sequence_process_refs.get(0).unwrap().process {
                            assert_eq!("pilte", pilte_sub_flow.alias(), "Sub-flow alias is not 'pilte' as expected");

                            if let Some(ref process_refs) = pilte_sub_flow.process_refs {
                                if let FunctionProcess(ref tap_function) = process_refs.get(0).unwrap().process {
                                    assert_eq!("tap", tap_function.alias(), "Function alias is not 'tap' as expected");
                                    if let Some(inputs) = tap_function.get_inputs() {
                                        let in_input = inputs.get(0).unwrap();
                                        assert_eq!("data", in_input.alias(), "Input's name is not 'data' as expected");
                                        assert_eq!("/context/sequence/pilte/tap/data", in_input.route(), "Input's route is not as expected");
                                        let initial_value = in_input.get_initial_value();
                                        match initial_value {
                                            Some(OneTime(one_time)) => assert_eq!(one_time.once, 1),
                                            _ => panic!("Initializer should have been a OneTime initializer")
                                        }
                                    } else {
                                        panic!("Could not find any inputs");
                                    }
                                } else {
                                    panic!("First sub-process of 'pass-if-lte' sub-flow was not a function as expected");
                                }
                            } else {
                                panic!("Could not get process_refs of pilte_sub_flow");
                            }
                        } else {
                            panic!("Second sub-process of sequence sub-flow was not another sub-flow as expected");
                        }
                    } else {
                        panic!("Could not get process_refs of sequence sub_flow");
                    }
                } else {
                    panic!("First sub-process of flow was not a sub-flow as expected");
                }
            },
            Ok(_) => panic!("Didn't load a flow"),
            Err(e) => panic!(e.to_string())
        }
    }
}
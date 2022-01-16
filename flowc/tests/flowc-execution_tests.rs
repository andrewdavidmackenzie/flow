#[macro_use]
extern crate error_chain;

use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::process::Stdio;

use serial_test::serial;
use simpath::Simpath;
use tempdir::TempDir;
use url::Url;

use flowclib::compiler::{compile, loader};
use flowclib::generator::generate;
use flowclib::generator::generate::GenerationTables;
use flowclib::model::flow::Flow;
use flowclib::model::process::Process;
use flowclib::model::process::Process::FlowProcess;
use flowcore::lib_provider::MetaProvider;

#[path = "helper.rs"]
mod helper;

#[doc(hidden)]
mod errors {
    // Create the Error, ErrorKind, ResultExt, and Result types
    error_chain! {}
}

error_chain! {
    foreign_links {
        Provider(::flowcore::errors::Error);
        Compiler(::flowclib::errors::Error);
        Io(::std::io::Error);
    }
}

/// Execution tests
///
/// These are a set of System tests that compile a sample flow, and then execute it, capturing
/// the output and comparing it to the expected output.

fn write_manifest(
    flow: &Flow,
    debug_symbols: bool,
    out_dir: PathBuf,
    test_name: &str,
    tables: &GenerationTables,
) -> Result<PathBuf> {
    let mut filename = out_dir;
    filename.push(&format!("{}.json", test_name));
    let mut manifest_file =
        File::create(&filename).chain_err(|| "Could not create manifest file")?;
    let out_dir_path =
        Url::from_file_path(&filename).map_err(|_| "Could not create filename url")?;

    let manifest = generate::create_manifest(
        &flow,
        debug_symbols,
        &out_dir_path,
        tables,
        HashSet::<(Url, Url)>::new(),
    )?;

    manifest_file
        .write_all(
            serde_json::to_string_pretty(&manifest)
                .chain_err(|| "Could not pretty format json for manifest")?
                .as_bytes(),
        )
        .chain_err(|| "Could not writ manifest data bytes to file")?;

    Ok(filename)
}

fn execute_flow(
    filepath: PathBuf,
    test_args: Vec<String>,
    input: String,
    client_server: bool,
) -> (String, String) {
    let server = if client_server {
        println!("Starting the 'flowr' server");
        let mut server_command = Command::new("cargo");
        let server_command_args = vec!["run", "--quiet", "-p", "flowr", "--", "-n", "-s"];

        // spawn the 'flowr' server child process
        Some(
            server_command
                .args(server_command_args)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap(),
        )
    } else {
        None
    };

    let mut command = Command::new("cargo");
    let mut command_args = vec!["run", "--quiet", "-p", "flowr", "--"];

    if client_server {
        // start another 'flowr' process in client mode
        command_args.push("-c");
        command_args.push("-a");
        command_args.push("localhost");
    } else {
        // when running client_and_server in same process we want to use native libs
        command_args.push("-n");
    }

    // Append the file path to the manifest to run to the command line args
    command_args.push(filepath.to_str().unwrap());

    // Append the flow arguments to the end of the command line
    for test_arg in &test_args {
        command_args.push(test_arg);
    }

    // spawn the 'flowr' child process
    let mut child = command
        .args(command_args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    // send it stdin from the "${testname}.stdin" file
    write!(child.stdin.unwrap(), "{}", input).unwrap();

    // read it's stdout
    let mut output = String::new();
    if let Some(ref mut stdout) = child.stdout {
        for line in BufReader::new(stdout).lines() {
            output.push_str(&format!("{}\n", &line.unwrap()));
        }
    }

    // read it's stderr
    let mut err = String::new();
    if let Some(ref mut stderr) = child.stderr {
        for line in BufReader::new(stderr).lines() {
            err.push_str(&format!("{}\n", &line.unwrap()));
        }
    }

    // If a server was started - then kill it
    if let Some(mut server_child) = server {
        println!("Killing 'flowr' server");
        server_child.kill().unwrap();
    }

    (output, err)
}

fn test_args(test_dir: &Path, test_name: &str) -> Vec<String> {
    let test_args = format!("{}.args", test_name);
    let mut args_file = test_dir.to_path_buf();
    args_file.push(test_args);
    let f = File::open(&args_file).unwrap();
    let f = BufReader::new(f);

    let mut args = Vec::new();
    for line in f.lines() {
        args.push(line.unwrap());
    }
    args
}

fn load_flow(test_dir: &Path, test_name: &str, search_path: Simpath) -> Process {
    let test_flow = format!("{}.toml", test_name);
    let mut flow_file = test_dir.to_path_buf();
    flow_file.push(test_flow);
    loader::load(
        &helper::absolute_file_url_from_relative_path(&flow_file.to_string_lossy()),
        &MetaProvider::new(search_path),
        &mut HashSet::<(Url, Url)>::new(),
    )
    .unwrap()
}

fn get(test_dir: &Path, file_name: &str) -> String {
    let mut expected_file = test_dir.to_path_buf();
    expected_file.push(file_name);
    let mut f = File::open(&expected_file).unwrap();
    let mut buffer = Vec::new();
    f.read_to_end(&mut buffer).unwrap();
    String::from_utf8(buffer).unwrap()
}

fn execute_test(test_name: &str, search_path: Simpath, client_server: bool) {
    let mut root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    root_dir.pop();
    let test_dir = root_dir.join(&format!("flowc/tests/test-flows/{}", test_name));

    if let FlowProcess(ref flow) = load_flow(&test_dir, test_name, search_path) {
        let tables = compile::compile(flow).unwrap();
        let dir =
            TempDir::new("flow").unwrap();
        let manifest_path = write_manifest(flow, true, dir.into_path(), test_name, &tables).unwrap();
        let test_args = test_args(&test_dir, test_name);
        let input = get(&test_dir, &format!("{}.stdin", test_name));
        let (actual_stdout, actual_stderr) =
            execute_flow(manifest_path, test_args, input, client_server);
        let expected_output = get(&test_dir, &format!("{}.expected", test_name));
        assert!(actual_stderr.is_empty(), "{}", actual_stderr);
        assert_eq!(
            expected_output, actual_stdout,
            "Flow output did not match that in .expected file"
        );
    }
}

#[cfg(feature = "debugger")]
#[test]
#[serial]
fn debug_print_args() {
    let search_path = helper::set_lib_search_path_to_project();
    execute_test("debug-print-args", search_path, false);
}

#[test]
#[serial]
fn print_args() {
    let search_path = helper::set_lib_search_path_to_project();
    execute_test("print-args", search_path, false);
}

#[test]
#[serial]
fn hello_world() {
    let search_path = helper::set_lib_search_path_to_project();
    execute_test("hello-world", search_path, false);
}

#[test]
#[serial]
fn line_echo() {
    let search_path = helper::set_lib_search_path_to_project();
    execute_test("line-echo", search_path, false);
}

#[test]
#[serial]
fn args() {
    let search_path = helper::set_lib_search_path_to_project();
    execute_test("args", search_path, false);
}

#[test]
#[serial]
fn args_json() {
    let search_path = helper::set_lib_search_path_to_project();
    execute_test("args_json", search_path, false);
}

#[test]
#[serial]
fn array_input() {
    let search_path = helper::set_lib_search_path_to_project();
    execute_test("array-input", search_path, false);
}

#[test]
#[serial]
fn double_connection() {
    let search_path = helper::set_lib_search_path_to_project();
    execute_test("double-connection", search_path, false);
}

#[test]
#[serial]
fn duplicate_connection() {
    let search_path = helper::set_lib_search_path_to_project();
    execute_test("duplicate-connection", search_path, false);
}

#[test]
#[serial]
fn two_destinations() {
    let search_path = helper::set_lib_search_path_to_project();
    execute_test("two_destinations", search_path, false);
}

#[cfg(feature = "distributed")]
#[test]
#[serial]
fn hello_world_client_server() {
    let search_path = helper::set_lib_search_path_to_project();
    execute_test("hello-world", search_path, true);
}

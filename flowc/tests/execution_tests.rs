#[macro_use]
extern crate error_chain;
extern crate flowclib;
extern crate flowrlib;
extern crate provider;
extern crate simpath;
extern crate url;

use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::io::prelude::*;
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;

use flowclib::compiler::compile;
use flowclib::compiler::loader;
use flowclib::generator::generate;
use flowclib::generator::generate::GenerationTables;
use flowclib::model::flow::Flow;
use flowclib::model::process::Process;
use flowclib::model::process::Process::FlowProcess;
use url::Url;

use provider::content::provider::MetaProvider;

mod errors {
    // Create the Error, ErrorKind, ResultExt, and Result types
    error_chain! {}
}

error_chain! {
    foreign_links {
        Provider(::provider::errors::Error);
        Compiler(::flowclib::errors::Error);
        Io(::std::io::Error);
    }
}

/// Execution tests
///
/// These are a set of System tests that compile a sample flow, and then execute it, capturing
/// the output and comparing it to the expected output.
fn url_from_rel_path(path: &str) -> String {
    let cwd = Url::from_file_path(env::current_dir().unwrap()).unwrap();
    let source_file = cwd.join(file!()).unwrap();
    let file = source_file.join(path).unwrap();
    file.to_string()
}

fn set_flow_lib_path() {
    let mut parent_dir = std::env::current_dir().unwrap();
    parent_dir.pop();
    println!("Set 'FLOW_LIB_PATH' to '{}'", parent_dir.to_string_lossy().to_string());
    env::set_var("FLOW_LIB_PATH", parent_dir.to_string_lossy().to_string());
}

fn write_manifest(flow: &Flow, debug_symbols: bool, out_dir: PathBuf, test_name: &str, tables: &GenerationTables)
                  -> Result<PathBuf> {
    let mut filename = out_dir.clone();
    filename.push(&format!("{}.json", test_name));
    let mut manifest_file = File::create(&filename).chain_err(|| "Could not create manifest file")?;
    let out_dir_path = Url::from_file_path(&filename).unwrap().to_string();

    let manifest = generate::create_manifest(&flow, debug_symbols, &out_dir_path, tables)?;

    manifest_file.write_all(serde_json::to_string_pretty(&manifest)
        .chain_err(|| "Could not pretty format json for manifest")?
        .as_bytes())
        .chain_err(|| "Could not writ emanifest data bytes to file")?;

    Ok(filename)
}

fn execute_flow(run_dir: PathBuf, filepath: PathBuf, test_args: Vec<String>, input: String) -> String {
    let mut command = Command::new("cargo");
    let mut command_args = vec!("run", "--bin", "flowr", "--", filepath.to_str().unwrap(),
                                "-j", "1", "-t", "0", "--");
    for test_arg in &test_args {
        command_args.push(test_arg);
    }

    println!("Command line: {:?}, {:?}", command, command_args);

    command.current_dir(run_dir);
    let mut child = command.args(command_args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn().unwrap();

    // send it stdin from the file
    write!(child.stdin.unwrap(), "{}", input).unwrap();

    let mut output = String::new();

    // read the stdout
    if let Some(ref mut stdout) = child.stdout {
        for line in BufReader::new(stdout).lines() {
            output.push_str(&format!("{}\n", &line.unwrap()));
        }
    }

    output
}

fn test_args(test_dir: &PathBuf, test_name: &str) -> Vec<String> {
    let test_args = format!("{}.args", test_name);
    let mut args_file = test_dir.clone();
    args_file.push(test_args);
    let f = File::open(&args_file).unwrap();
    let f = BufReader::new(f);

    let mut args = Vec::new();
    for line in f.lines() {
        args.push(line.unwrap());
    }
    println!("flow args: {:?}", args);
    args
}

fn load_flow(test_dir: &PathBuf, test_name: &str) -> Process {
    let test_flow = format!("{}.toml", test_name);
    let mut flow_file = test_dir.clone();
    flow_file.push(test_flow);
    loader::load_context(&url_from_rel_path(&flow_file.to_string_lossy()), &MetaProvider {}).unwrap()
}

fn get(test_dir: &PathBuf, file_name: &str) -> String {
    let mut expected_file = test_dir.clone();
    expected_file.push(file_name);
    let mut f = File::open(&expected_file).unwrap();
    let mut buffer = Vec::new();
    f.read_to_end(&mut buffer).unwrap();
    String::from_utf8(buffer).unwrap()
}

fn execute_test(test_name: &str) {
    set_flow_lib_path();
    let mut test_dir = env::current_dir().unwrap();
    let mut run_dir = test_dir.clone();
    run_dir.pop();
    test_dir.push(&format!("tests/samples/{}", test_name));

    if let FlowProcess(ref flow) = load_flow(&test_dir, test_name) {
        let tables = compile::compile(flow).unwrap();
        let out_dir = test_dir.clone();
        let manifest_path = write_manifest(flow, true, out_dir,
                                           test_name, &tables).unwrap();

        let test_args = test_args(&test_dir, test_name);
        let input = get(&test_dir, &format!("{}.stdin", test_name));
        let actual_output = execute_flow(run_dir, manifest_path, test_args, input);
        let expected_output = get(&test_dir, &format!("{}.expected", test_name));
        assert_eq!(expected_output, actual_output, "Flow output did not match that in .expected file");
    }
}

#[test]
fn print_args() {
    execute_test("print-args");
}

#[test]
fn primitives() {
    execute_test("primitives");
}

#[test]
fn hello_world() {
    execute_test("hello-world");
}

#[test]
fn echo() {
    execute_test("line-echo");
}

#[test]
fn range() {
    execute_test("range");
}
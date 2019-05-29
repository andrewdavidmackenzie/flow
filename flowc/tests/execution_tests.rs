extern crate flowclib;
extern crate flowrlib;
extern crate provider;
extern crate simpath;
extern crate url;

use std::env;
use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;
use std::path::PathBuf;
use std::process::Command;
use std::process::Output;
use std::process::Stdio;

use flowclib::compiler::compile;
use flowclib::compiler::loader;
use flowclib::generator::generate;
use flowclib::generator::generate::GenerationTables;
use flowclib::model::flow::Flow;
use flowclib::model::process::Process;
use flowclib::model::process::Process::FlowProcess;
use flowrlib::manifest::DEFAULT_MANIFEST_FILENAME;
use url::Url;

use provider::content::provider::MetaProvider;

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

fn write_manifest(flow: &Flow, debug_symbols: bool, out_dir: PathBuf, tables: &GenerationTables)
                  -> Result<PathBuf, std::io::Error> {
    let mut filename = out_dir.clone();
    filename.push(DEFAULT_MANIFEST_FILENAME.to_string());
    let mut manifest_file = File::create(&filename)?;
    let out_dir_path = Url::from_file_path(out_dir).unwrap().to_string();

    let manifest = generate::create_manifest(&flow, debug_symbols, &out_dir_path, tables)?;

    manifest_file.write_all(serde_json::to_string_pretty(&manifest)?.as_bytes())?;

    Ok(filename)
}

fn execute_flow(run_dir: PathBuf, filepath: PathBuf, test_args: Vec<String>) -> Output {
    let mut command = Command::new("cargo");
    let mut command_args = vec!("run", "--bin", "flowr", filepath.to_str().unwrap(),
                            "--");
    test_args.iter().map(|arg| command_args.push(arg));
    command.current_dir(run_dir);
    command.args(command_args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::piped())
        .output().unwrap()
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
    args
}

fn load_flow(test_dir: &PathBuf, test_name: &str) -> Process {
    let test_flow = format!("{}.toml", test_name);
    let mut flow_file = test_dir.clone();
    flow_file.push(test_flow);
    loader::load_context(&url_from_rel_path(&flow_file.to_string_lossy()), &MetaProvider {}).unwrap()
}

fn get(file_path: &PathBuf) -> Vec<u8> {
    let mut f = File::open(&file_path).unwrap();
    let mut buffer = Vec::new();
    f.read_to_end(&mut buffer).unwrap();
    buffer
}

fn execute_test(test_name: &str) {
    set_flow_lib_path();
    let mut test_dir = env::current_dir().unwrap();
    let mut run_dir = test_dir.clone();
    run_dir.pop();
    test_dir.push("tests/samples");

    if let FlowProcess(ref flow) = load_flow(&test_dir, test_name) {
        let tables = compile::compile(flow).unwrap();
        let out_dir = test_dir.clone();
        let manifest_path = write_manifest(flow, true, out_dir, &tables).unwrap();

        let test_stdin = format!("{}.stdin", test_name);
        let mut test_args = test_args(&test_dir, test_name);
        let output = execute_flow(run_dir, manifest_path, test_args);
    }
}

#[test]
fn args() {
    execute_test("print_args");
}
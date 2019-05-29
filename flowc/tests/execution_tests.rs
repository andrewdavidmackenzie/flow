extern crate flowclib;
extern crate flowrlib;
extern crate provider;
extern crate simpath;
extern crate url;

use std::env;
use std::fs::File;
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

fn execute_flow(run_dir: PathBuf, filepath: PathBuf) -> Output {
    let mut command = Command::new("cargo");
    let command_args = vec!("run", "--bin", "flowr", filepath.to_str().unwrap(),
                            "--", "arg1");
    command.current_dir(run_dir);
    command.args(command_args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::piped())
        .output().unwrap()
}

fn test_args(test_name: &str) -> Vec<&str> {
    let test_args = format!("{}.args", test_name);

    // TODO read args from file

    vec!()
}

fn load_flow(test_dir: &mut PathBuf, test_name: &str) -> Process {
    let test_flow = format!("{}.toml", test_name);
    test_dir.push(test_flow);
    loader::load_context(&url_from_rel_path(&test_dir.to_string_lossy()), &MetaProvider {}).unwrap()
}

fn execute_test(test_name: &str) {
    set_flow_lib_path();
    let mut test_dir = env::current_dir().unwrap();
    let mut run_dir = test_dir.clone();
    run_dir.pop();
    test_dir.push("tests/samples");
    let out_dir = test_dir.clone();
    let test_stdin = format!("{}.stdin", test_name);
    let test_args = test_args(test_name);

    if let FlowProcess(ref flow) = load_flow(&mut test_dir, test_name) {
        let tables = compile::compile(flow).unwrap();
        let manifest_path = write_manifest(flow, true, out_dir, &tables).unwrap();
        let output = execute_flow(run_dir, manifest_path);
    }
}

#[test]
fn args() {
    execute_test("print_args");
}
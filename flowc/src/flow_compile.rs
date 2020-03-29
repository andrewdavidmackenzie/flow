use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;

use log::{debug, error, info};
use simpath::FileType;
use simpath::Simpath;
use url::Url;

use flowclib::compiler::compile;
use flowclib::compiler::loader;
use flowclib::dumper::dump_flow;
use flowclib::dumper::dump_tables;
use flowclib::generator::generate;
use flowclib::generator::generate::GenerationTables;
use flowclib::model::flow::Flow;
use flowclib::model::name::HasName;
use flowclib::model::process::Process::FlowProcess;
use flowrlib::manifest::DEFAULT_MANIFEST_FILENAME;
use flowrlib::provider::Provider;

use crate::compile_wasm;
use crate::errors::*;

/*
    Check root process fits the rules for a Context and being a runnable flow
*/
fn check_root(flow: &Flow) -> bool {
    let mut runnable = true;

    if let Some(inputs) = flow.inputs() {
        if !inputs.is_empty() {
            error!("Root process '{}' has inputs:", flow.name);
            for input in inputs {
                error!("\t'{}'", input.name());
            }
            runnable = false
        }
    }

    if let Some(outputs) = flow.outputs() {
        if !outputs.is_empty() {
            error!("Root process '{}' has outputs:", flow.name);
            for output in outputs {
                error!("\t'{}'", output.name());
            }
            runnable = false
        }
    }

    runnable
}

/*
    Compile a flow, maybe run it
*/
pub fn compile_and_execute_flow(url: &Url, flow_args: Vec<String>, dump: bool, skip_generation: bool, debug_symbols: bool,
                                provided_implementations: bool, out_dir: PathBuf, provider: &dyn Provider, release: bool)
                                -> Result<String> {
    info!("==== Compiler phase: Loading flow");
    let context = loader::load_root(&url.to_string(), provider)
        .chain_err(|| "Couldn't load context")?;
    match context {
        FlowProcess(flow) => {
            let mut tables = compile::compile(&flow)
                .chain_err(|| format!("Could not compile the flow '{}'", url))?;

            compile_supplied_implementations(&mut tables, provided_implementations, release)?;

            let runnable = check_root(&flow);

            if dump {
                dump_flow::dump_flow(&flow, &out_dir)
                    .chain_err(|| "Failed to dump flow's definition")?;
                dump_tables::dump_tables(&tables, &out_dir)
                    .chain_err(|| "Failed to dump flow's tables")?;
                dump_tables::dump_functions(&flow, &tables, &out_dir)
                    .chain_err(|| "Failed to dump flow's functions")?;
            }

            if !runnable {
                return Ok("Flow not runnable, so Manifest generation and execution skipped".to_string());
            }

            if skip_generation {
                return Ok("Manifest generation and execution skipped".to_string());
            }

            info!("==== Compiler phase: Generating Manifest");
            let manifest_path = write_flow_manifest(flow, debug_symbols, out_dir, &tables)
                .chain_err(|| "Failed to write manifest")?;

            info!("==== Compiler phase: Executing flow from manifest");
            execute_flow(manifest_path, flow_args)
        }
        _ => bail!("Process loaded was not of type 'Flow' and cannot be executed")
    }
}

/*
    For any function that provides an implementation - compile the source to wasm and modify the
    implementation to indicate it is the wasm file
*/
fn compile_supplied_implementations(tables: &mut GenerationTables, skip_building: bool, release: bool) -> Result<String> {
    for function in &mut tables.functions {
        if let Some(_) = function.get_implementation() {
            compile_wasm::compile_implementation(function, skip_building, release)?;
        };
    }

    Ok("All supplied implementations compiled successfully".into())
}

/*
    Generate a manifest for the flow in JSON that can be used to run it using 'flowr'
*/
fn write_flow_manifest(flow: Flow, debug_symbols: bool, out_dir_path: PathBuf, tables: &GenerationTables)
                       -> Result<PathBuf> {
    let mut filename = out_dir_path.clone();
    filename.push(DEFAULT_MANIFEST_FILENAME.to_string());
    filename.set_extension("json");
    let mut manifest_file = File::create(&filename).chain_err(|| "Could not create manifest file")?;
    let manifest = generate::create_manifest(&flow, debug_symbols, out_dir_path.to_str()
        .ok_or("Could not convert output directory to string")?, tables)
        .chain_err(|| "Could not create manifest from parsed flow and compiler tables")?;

    manifest_file.write_all(serde_json::to_string_pretty(&manifest)
        .chain_err(|| "Could not pretty format the manifest JSON contents")?
        .as_bytes()).chain_err(|| "Could not write manifest data bytes to created manifest file")?;

    Ok(filename)
}

#[cfg(not(target_os = "windows"))]
fn get_executable_name() -> String {
    "flowr".to_string()
}

#[cfg(target_os = "windows")]
fn get_executable_name() -> String {
    "flowr.exe".to_string()
}

/*
    Find the absolute path to the executable to be used to run the flow.
        - First looking for development directories under the Current Working Directory
          to facilitate development.
        - If not found there, then look in the PATH env variable
*/
fn find_executable_path(name: &str) -> Result<String> {
    // See if debug version in development is available
    let cwd = env::current_dir().chain_err(|| "Could not get the current working directory")?;
    let file = cwd.join(&format!("./target/debug/{}", name));
    let abs_path = file.canonicalize();
    if let Ok(file_exists) = abs_path {
        return Ok(file_exists.to_string_lossy().to_string());
    }

    // Couldn't find the development version under CWD where running, so look in path
    let bin_search_path = Simpath::new("PATH");
    let bin_path = bin_search_path.find_type(name, FileType::File)
        .chain_err(|| format!("Could not find executable '{}'", name))?;

    Ok(bin_path.to_string_lossy().to_string())
}

/*
    Run flow using 'flowr'
    Inherit standard output and input and just let the process run as normal.
    Capture standard error.
    If the process exits correctly then just return an Ok() with message and no log
    If the process fails then return an Err() with message and log stderr in an ERROR level message
*/
fn execute_flow(filepath: PathBuf, mut flow_args: Vec<String>) -> Result<String> {
    info!("Executing flow from manifest in '{}'", filepath.display());

    let command = find_executable_path(&get_executable_name())?;
    let mut command_args = vec!(filepath.to_str().unwrap().to_string());
    if !flow_args.contains(&"-n".to_string()) {
        command_args.push("-n".to_string());
    }
    command_args.append(&mut flow_args);
    debug!("Running flow using '{} {:?}'", &command, &command_args);
    let output = Command::new(&command).args(command_args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output().chain_err(|| "Error while attempting to spawn command to compile and run flow")?;
    match output.status.code() {
        Some(0) => Ok("".into()),
        Some(code) => {
            error!("Execution of 'flowr' failed");
            error!("Process STDOUT:\n{}", String::from_utf8_lossy(&output.stdout));
            error!("Process STDERR:\n{}", String::from_utf8_lossy(&output.stderr));
            bail!("Exited with status code: {}", code)
        }
        None => Ok("No return code - ignoring".to_string())
    }
}
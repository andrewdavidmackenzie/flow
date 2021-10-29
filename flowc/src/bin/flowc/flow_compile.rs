#[cfg(feature = "debugger")]
use std::collections::HashSet;
use std::env;
use std::path::Path;
use std::process::Command;
use std::process::Stdio;

use log::{debug, error, info};
use simpath::Simpath;
use simpath::{FileType, FoundType};
#[cfg(feature = "debugger")]
use url::Url;

use flowclib::compiler::compile;
use flowclib::compiler::compile_wasm;
use flowclib::compiler::loader;
use flowclib::dumper::dump_flow;
use flowclib::dumper::dump_tables;
use flowclib::generator::generate;
use flowclib::generator::generate::GenerationTables;
use flowclib::model::flow::Flow;
use flowclib::model::process::Process::FlowProcess;
use flowcore::lib_provider::Provider;

use crate::errors::*;
use crate::Options;

/*
    Check root process fits the rules for a Context and being a runnable flow
*/
fn check_root(flow: &Flow) -> bool {
    let mut runnable = true;

    if !flow.inputs().is_empty() {
        error!(
            "Root process '{}' has inputs: {:?}",
            flow.name,
            flow.inputs()
        );
        runnable = false
    }

    if !flow.outputs().is_empty() {
        error!(
            "Root process '{}' has outputs: {:?}",
            flow.name,
            flow.outputs()
        );
        runnable = false
    }

    runnable
}

// For any function that provides an implementation - compile the source to wasm and modify the
// implementation to indicate it is the wasm file
fn compile_supplied_implementations(
    _out_dir: &Path,
    tables: &mut GenerationTables,
    skip_building: bool,
    #[cfg(feature = "debugger")] source_urls: &mut HashSet<(Url, Url)>,
) -> Result<String> {
    for function in &mut tables.functions {
        if function.get_lib_reference().is_none() {
            // calculate directory where we want compiled implementation to be left
            let source_dir = function
                .get_source_url()
                .to_file_path()
                .map_err(|_| "Could not get file path for function source")?
                .parent()
                .ok_or("Couldn't get directory where function defined")?
                .to_path_buf();

            compile_wasm::compile_implementation(
                &source_dir,
                function,
                skip_building,
                #[cfg(feature = "debugger")]
                source_urls,
            )?;
        }
    }

    Ok("All supplied implementations compiled successfully".into())
}

/*
    Compile a flow, maybe run it
*/
pub fn compile_and_execute_flow(options: &Options, provider: &dyn Provider) -> Result<String> {
    info!("==== Compiler phase: Loading flow");
    #[cfg(feature = "debugger")]
    let mut source_urls = HashSet::<(Url, Url)>::new();
    let context = loader::load(
        &options.source_url,
        provider,
        #[cfg(feature = "debugger")]
        &mut source_urls,
    )
    .chain_err(|| format!("Could not load flow from '{}'", options.source_url))?;

    match context {
        FlowProcess(flow) => {
            let mut tables = compile::compile(&flow)
                .chain_err(|| format!("Could not compile flow from '{}'", options.source_url))?;

            compile_supplied_implementations(
                &options.output_dir,
                &mut tables,
                options.provided_implementations,
                #[cfg(feature = "debugger")]
                &mut source_urls,
            )
            .chain_err(|| "Could not compile supplied implementation to wasm")?;

            let runnable = check_root(&flow);

            dump(&flow, provider, &tables, options)?;

            if !runnable {
                return Ok(
                    "Flow not runnable, so Manifest generation and execution skipped".to_string(),
                );
            }

            info!("==== Compiler phase: Generating Manifest");
            let manifest_path = generate::write_flow_manifest(
                flow,
                options.debug_symbols,
                &options.output_dir,
                &tables,
                #[cfg(feature = "debugger")]
                source_urls,
            )
            .chain_err(|| "Failed to write manifest")?;

            if options.skip_execution {
                return Ok("Flow execution skipped".to_string());
            }

            info!("==== Compiler phase: Executing flow from manifest");
            execute_flow(&manifest_path, options)
        }
        _ => bail!("Process loaded was not of type 'Flow' and cannot be executed"),
    }
}

fn dump(
    flow: &Flow,
    provider: &dyn Provider,
    tables: &GenerationTables,
    options: &Options,
) -> Result<String> {
    if options.dump || options.graphs {
        dump_flow::dump_flow(
            flow,
            &options.output_dir,
            provider,
            options.dump,
            options.graphs,
        )
        .chain_err(|| "Failed to dump flow's definition")?;

        if options.graphs {
            dump_flow::generate_svgs(&options.output_dir)?;
        }

        if options.dump {
            dump_tables::dump_tables(tables, &options.output_dir)
                .chain_err(|| "Failed to dump flow's tables")?;
            dump_tables::dump_functions(flow, tables, &options.output_dir)
                .chain_err(|| "Failed to dump flow's functions")?;
        }
    }

    Ok("Dumped".into())
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

    let file = cwd.join(&format!("./target/release/{}", name));
    let abs_path = file.canonicalize();
    if let Ok(file_exists) = abs_path {
        return Ok(file_exists.to_string_lossy().to_string());
    }

    // Couldn't find the development version under CWD where running, so look in path
    let bin_search_path = Simpath::new("PATH");
    match bin_search_path.find_type(name, FileType::File) {
        Ok(FoundType::File(bin_path)) => Ok(bin_path.to_string_lossy().to_string()),
        _ => bail!("Could not find executable '{}'", name),
    }
}

/*
    Run flow using 'flowr'
    Inherit standard output and input and just let the process run as normal.
    Capture standard error.
    If the process exits correctly then just return an Ok() with message and no log
    If the process fails then return an Err() with message and log stderr in an ERROR level message
*/
fn execute_flow(filepath: &Path, options: &Options) -> Result<String> {
    info!("Executing flow from manifest in '{}'", filepath.display());

    let command = find_executable_path(&get_executable_name())?;
    let mut command_args = vec![filepath.display().to_string()];

    // If the user didn't already specify the "-n" (native libraries) option for execution of
    // "flowr" then add it - to execute flows using the libraries natively linked to "flow"
    if !options.flow_args.contains(&"-n".to_string()) {
        command_args.push("-n".to_string());
    }

    if !options.flow_args.is_empty() {
        command_args.push("--".to_string());
        command_args.append(&mut options.flow_args.to_vec());
    }
    println!("Running flow using '{} {:?}'", &command, &command_args);

    let mut flowr = Command::new(&command);
    flowr
        .args(command_args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    if options.stdin_file.is_some() {
        flowr.stdin(Stdio::piped());
    }

    let mut flowr_child = flowr.spawn().chain_err(|| "Could not spawn 'flowr'")?;

    if let Some(stdin_file) = &options.stdin_file {
        debug!("Reading STDIN from file: '{}'", stdin_file);

        let _ = Command::new("cat")
            .args(vec![stdin_file])
            .stdout(
                flowr_child
                    .stdin
                    .take()
                    .chain_err(|| "Could not read child process stdin")?,
            )
            .spawn()
            .chain_err(|| "Could not spawn 'cat' to pipe STDIN to 'flowr'");
    }

    let flowr_output = flowr_child
        .wait_with_output()
        .chain_err(|| "Could not capture 'flowr' output")?;

    match flowr_output.status.code() {
        Some(0) => Ok("".into()),
        Some(code) => {
            error!("Execution of 'flowr' failed");
            error!(
                "Process STDOUT:\n{}",
                String::from_utf8_lossy(&flowr_output.stdout)
            );
            error!(
                "Process STDERR:\n{}",
                String::from_utf8_lossy(&flowr_output.stderr)
            );
            bail!("Exited with status code: {}", code)
        }
        None => Ok("No return code - ignoring".to_string()),
    }
}

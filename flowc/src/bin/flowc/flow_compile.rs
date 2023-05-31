#[cfg(feature = "debugger")]
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::process::Stdio;

use log::{debug, error, info};
#[cfg(feature = "debugger")]
use url::Url;

use flowcore::model::process::Process::FlowProcess;
use flowcore::provider::Provider;
use flowrclib::compiler::compile;
use flowrclib::compiler::parser;
use flowrclib::dumper::{flow_to_dot, functions_to_dot};
use flowrclib::generator::generate;

use crate::errors::*;
use crate::Options;

/// Compile a flow, maybe run it
pub fn compile_and_execute_flow(options: &Options, provider: &dyn Provider,
                                runner_name: String) -> Result<()> {
    info!("==== Parsing flow hierarchy from '{}'", options.source_url);
    #[cfg(feature = "debugger")]
    let mut source_urls = BTreeMap::<String, Url>::new();

    let root = parser::parse(
        &options.source_url,
        provider,
    )?;

    match root {
        FlowProcess(flow) => {
            info!("Finished parsing flow hierarchy starting at root flow '{}'", flow.name);
            let tables = compile::compile(&flow,
                                              &options.output_dir,
                                              options.provided_implementations,
                                              options.optimize,
                                                &mut source_urls
            ).chain_err(|| format!("Could not compile the flow '{}'", options.source_url))?;

            make_writeable(&options.output_dir)?;

            if options.graphs {
                flow_to_dot::dump_flow(&flow, &options.output_dir, provider)?;
                functions_to_dot::dump_functions(&flow, &tables, &options.output_dir)?;
                flow_to_dot::generate_svgs(&options.output_dir, true)?;
            }

            if !flow.is_runnable() {
                info!("Flow not runnable, so Manifest generation and flow execution skipped");
                return Ok(());
            }

            let manifest_path = generate::write_flow_manifest(
                flow,
                options.debug_symbols,
                &options.output_dir,
                &tables,
                #[cfg(feature = "debugger")] source_urls,
            )
            .chain_err(|| "Failed to write manifest")?;

            if options.compile_only {
                info!("Flow execution skipped");
                return Ok(());
            }

            execute_flow(&manifest_path, options, runner_name)
        }
        _ => bail!("Process parsed was not of type 'Flow' and cannot be executed"),
    }
}

// Make sure the directory exists, if not create it, and is writable
fn make_writeable(output_dir: &PathBuf) -> Result<()> {
    if output_dir.exists() {
        // Check it's not a file!
        if output_dir.is_file() {
            bail!(
                "Output directory '{}' already exists as a file",
                output_dir.display()
            );
        }

        let md = fs::metadata(output_dir)?;

        // check it's not read only!
        if md.permissions().readonly() {
            bail!("Output directory '{}' is read only", output_dir.display());
        }
    } else {
        fs::create_dir_all(output_dir)
            .chain_err(|| format!("Could not create directory '{}'", output_dir.display()))?;
    }

    Ok(())
}

/*
    Inherit standard output and input and just let the process run as normal.
    Capture standard error.
    If the process exits correctly then just return an Ok() with message and no log
    If the process fails then return an Err() with message and log stderr in an ERROR level message
*/
fn execute_flow(filepath: &Path, options: &Options, runner_name: String) -> Result<()> {
    info!("\n==== Executing flow from manifest at '{}'", filepath.display());

    let mut runner_args = vec![];

    // if a specific verbosity level was set on the Command Line to flowc, pass it on to runner
    if let Some(verbosity) = &options.verbosity {
        runner_args.push("-v".to_string());
        runner_args.push(verbosity.to_string());
    }

    // if execution metrics requested to flowc, pass that onto runner
    if options.execution_metrics {
        runner_args.push("-m".to_string());
    }

    // if debug (symbols) requested to flowc, pass that onto runner
    if options.debug_symbols {
        runner_args.push("-d".to_string());
    }

    // unless wasm execution requested, pass the native flag onto runner
    if !options.wasm_execution {
        runner_args.push("-n".to_string());
    }

    // pass along any specified library directories to runner
    for lib_dir in &options.lib_dirs {
        runner_args.push("-L".to_string());
        runner_args.push(lib_dir.to_string());
    }

    runner_args.push(filepath.display().to_string());

    // any arguments for the flow itself (not runner) go at the end
    runner_args.append(&mut options.flow_args.to_vec());

    info!("Running flow using '{} {:?}'", runner_name, &runner_args);
    let mut runner = Command::new(&runner_name);
    runner
        .args(runner_args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    if options.stdin_file.is_some() {
        runner.stdin(Stdio::piped());
    }

    let mut runner_child = runner.spawn()
        .chain_err(|| format!("Could not spawn '{}'", runner_name))?;

    if let Some(stdin_file) = &options.stdin_file {
        debug!("Reading STDIN from file: '{}'", stdin_file);

        let _ = Command::new("cat")
            .args(vec![stdin_file])
            .stdout(
                runner_child
                    .stdin
                    .take()
                    .chain_err(|| "Could not read child process stdin")?,
            )
            .spawn()
            .chain_err(|| format!("Could not spawn 'cat' to pipe STDIN to '{}'", runner_name));
    }

    let runner_output = runner_child
        .wait_with_output()
        .chain_err(|| format!("Could not capture '{}' output", runner_name))?;

    match runner_output.status.code() {
        Some(0) => Ok(()),
        Some(code) => {
            error!("Execution of '{}' failed", runner_name);
            error!("'{}' STDOUT:\n{}", runner_name, String::from_utf8_lossy(&runner_output.stdout));
            error!("'{}' STDERR:\n{}", runner_name, String::from_utf8_lossy(&runner_output.stderr));
            bail!("Execution of '{}' failed. Exited with status code: {}", runner_name, code)
        }
        None => Ok(()),
    }
}

#[cfg(test)]
mod test {
    use std::fs;

    use tempdir::TempDir;

    use crate::flow_compile::make_writeable;

    #[test]
    fn can_create_dir_correctly() {
        let temp_parent = TempDir::new("flow_compile")
            .expect("Could not create temp parent dir");

        let test_output_dir = temp_parent.path().join("output");
        make_writeable(&test_output_dir).expect("Could not make output dir");

        assert!(test_output_dir.exists());
        assert!(test_output_dir.is_dir());
        let md = fs::metadata(test_output_dir).expect("Could not get metadata");
        assert!(!md.permissions().readonly());
    }

    #[test]
    fn can_use_existing_dir() {
        let test_output_dir = TempDir::new("flow_compile")
            .expect("Could not create temp parent dir").into_path();

        make_writeable(&test_output_dir).expect("Could not make output dir");

        assert!(test_output_dir.exists());
        assert!(test_output_dir.is_dir());
        let md = fs::metadata(test_output_dir).expect("Could not get metadata");
        assert!(!md.permissions().readonly());
    }

    #[test]
    fn error_if_exists_as_file() {
        let temp_parent = TempDir::new("flow_compile")
            .expect("Could not create temp parent dir");
        let test_output_file = temp_parent.path().join("output");
        fs::File::create(&test_output_file).expect("Could not create file");

        assert!(make_writeable(&test_output_file).is_err());
    }
}
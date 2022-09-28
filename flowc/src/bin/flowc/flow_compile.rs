#[cfg(feature = "debugger")]
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::process::Stdio;

use log::{debug, error, info};
#[cfg(feature = "debugger")]
use url::Url;

use flowclib::compiler::compile;
use flowclib::compiler::compile::CompilerTables;
use flowclib::compiler::parser;
use flowclib::dumper::{dump, dump_dot};
use flowclib::generator::generate;
use flowcore::meta_provider::Provider;
use flowcore::model::flow_definition::FlowDefinition;
use flowcore::model::process::Process::FlowProcess;

use crate::errors::*;
use crate::Options;

/// Compile a flow, maybe run it
pub fn compile_and_execute_flow(options: &Options, provider: &dyn Provider) -> Result<()> {
    info!("==== Parsing flow hierarchy from '{}'", options.source_url);
    #[cfg(feature = "debugger")]
    let mut source_urls = BTreeSet::<(Url, Url)>::new();

    let root = parser::parse(
        &options.source_url,
        provider,
        #[cfg(feature = "debugger")]
        &mut source_urls,
    )?;

    match root {
        FlowProcess(flow) => {
            info!("Finished parsing flow hierarchy starting at root flow '{}'", flow.name);
            let tables = compile::compile(&flow,
                                              &options.output_dir,
                                              options.provided_implementations,
                                              options.optimize,
                                              #[cfg(feature = "debugger")] &mut source_urls,
            ).chain_err(|| format!("Could not compile the flow '{}'", options.source_url))?;

            make_writeable(&options.output_dir)?;

            dump(&flow, provider, &tables, options)?;

            if !flow.is_runnable() {
                info!("Flow not runnable, so Manifest generation and flow execution skipped");
                return Ok(());
            }

            let manifest_path = generate::write_flow_manifest(
                flow,
                options.debug_symbols,
                &options.output_dir,
                &tables,
                #[cfg(feature = "debugger")] &source_urls,
            )
            .chain_err(|| "Failed to write manifest")?;

            if options.compile_only {
                info!("Flow execution skipped");
                return Ok(());
            }

            execute_flow(&manifest_path, options)
        }
        _ => bail!("Process parsed was not of type 'Flow' and cannot be executed"),
    }
}

fn make_writeable(output_dir: &PathBuf) -> Result<()> {
    // Now make sure the directory exists, if not create it, and is writable
    if output_dir.exists() {
        let md = fs::metadata(output_dir).chain_err(|| {
            format!(
                "Could not read metadata of the existing output directory '{}'",
                output_dir.display()
            )
        })?;
        // Check it's not a file!
        if md.is_file() {
            bail!(
                "Output directory '{}' already exists as a file",
                output_dir.display()
            );
        }

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

fn dump(
    flow: &FlowDefinition,
    provider: &dyn Provider,
    tables: &CompilerTables,
    options: &Options,
) -> Result<()> {
    if options.tables_dump {
        dump::dump_flow(
            flow,
            &options.output_dir,
            provider
        ).chain_err(|| "Failed to dump flow definition")?;

        dump::dump_tables(tables, &options.output_dir)
            .chain_err(|| "Failed to dump the compiled flow's compiler tables")?;

        dump::dump_functions(flow, tables, &options.output_dir)
            .chain_err(|| "Failed to dump the compiled flow's functions")?;
    }

    if options.graphs {
        dump_dot::dump_flow(flow, &options.output_dir, provider)?;
        dump_dot::dump_functions(flow, tables, &options.output_dir)?;
        dump_dot::generate_svgs(&options.output_dir, true)?;
    }

    Ok(())
}

/*
    Run flow using 'flowr'
    Inherit standard output and input and just let the process run as normal.
    Capture standard error.
    If the process exits correctly then just return an Ok() with message and no log
    If the process fails then return an Err() with message and log stderr in an ERROR level message
*/
fn execute_flow(filepath: &Path, options: &Options) -> Result<()> {
    info!("\n==== Executing flow from manifest at '{}'", filepath.display());

    let mut flowr_args = vec![];

    // if a specific verbosity level was set on the CL to flowc, pass it on to flowr
    if let Some(verbosity) = &options.verbosity {
        flowr_args.push("-v".to_string());
        flowr_args.push(verbosity.to_string());
    }

    // if execution metrics requested to flowc, pass that onto flowr
    if options.execution_metrics {
        flowr_args.push("-m".to_string());
    }

    // if debug (symbols) requested to flowc, pass that onto flowr
    if options.debug_symbols {
        flowr_args.push("-d".to_string());
    }

    // unless wasm execution requested, pass the native flag onto flowr
    if !options.wasm_execution {
        flowr_args.push("-n".to_string());
    }

    // pass along any specified library directories to flowr also
    for lib_dir in &options.lib_dirs {
        flowr_args.push("-L".to_string());
        flowr_args.push(lib_dir.to_string());
    }

    flowr_args.push(filepath.display().to_string());

    // any arguments for the flow itself (not flowr) go at the end
    flowr_args.append(&mut options.flow_args.to_vec());

    info!("Running flow using 'flowr {:?}'", &flowr_args);
    let mut flowr = Command::new("flowr");
    flowr
        .args(flowr_args)
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
        Some(0) => Ok(()),
        Some(code) => {
            error!("Execution of 'flowr' failed");
            error!(
                "flowr STDOUT:\n{}",
                String::from_utf8_lossy(&flowr_output.stdout)
            );
            error!(
                "flowr STDERR:\n{}",
                String::from_utf8_lossy(&flowr_output.stderr)
            );
            bail!(
                "Execution of flowr failed. Exited with status code: {}",
                code
            )
        }
        None => Ok(()),
    }
}

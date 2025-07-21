use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use colored::Colorize;
use log::debug;

use crate::errors::{Result, ResultExt, bail};

/*
   Check the command Output for an error and print details if it failed
*/
fn check_cargo_error(command: &str, args: &[&str], output: &Output) -> Result<()> {
    match output.status.code() {
        Some(0) | None => Ok(()),
        Some(code) => {
            println!("Command Line: {command} {}", args.join(" "));
            if !&output.stdout.is_empty() {
                println!(
                    "{}\n{}",
                    "STDOUT:".green(),
                    String::from_utf8_lossy(&output.stdout).green()
                );
            }
            if !&output.stderr.is_empty() {
                eprintln!(
                    "{}\n{}",
                    "STDERR:".red(),
                    String::from_utf8_lossy(&output.stderr).red()
                );
            }
            bail!("cargo exited with status code: {}", code)
        }
    }
}

fn cargo_test(manifest_path: &Path) -> Result<()> {
    let command = "cargo";

    let manifest_arg = format!("--manifest-path={}", manifest_path.display());
    let test_args = vec!["test", &manifest_arg];

    println!(
        "   {} {} WASM Project",
        "Testing".green(),
        manifest_path.display()
    );

    debug!("\tRunning command = '{command}', args = {test_args:?}");

    let output = Command::new(command)
        .args(&test_args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .chain_err(|| "Error while attempting to spawn cargo to test WASM Project")?;

    check_cargo_error(command, &test_args, &output)
}

/*
   Run the cargo command that builds the WASM output file
*/
fn cargo_build(
    manifest_path: &Path,
    mut cargo_target_dir: PathBuf, // where the binary is to be built by cargo
    release_build: bool,
    implementation_source_path: &Path,
    wasm_destination: &Path,
) -> Result<()> {
    let command = "cargo";
    let manifest = format!("--manifest-path={}", manifest_path.display());

    println!(
        "   {} {} WASM project",
        "Compiling".green(),
        manifest_path.display()
    );

    debug!(
        "\tBuilding WASM '{}' from source '{}'",
        wasm_destination.display(),
        implementation_source_path.display()
    );

    let mut command_args = vec!["build"];

    if release_build {
        command_args.push("--release");
    }

    command_args.append(&mut vec![
        "--lib",
        "--target=wasm32-unknown-unknown",
        &manifest,
        ]
    );

    debug!("\tRunning command = '{command}', command_args = {command_args:?}");

    let output = Command::new(command)
        .args(&command_args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env_remove("RUSTFLAGS") // remove flags for coverage, incompatible with wasm build
        .env_remove("CARGO_BUILD_RUSTFLAGS") // remove flags for coverage, incompatible with wasm build
        .env_remove("CARGO_ENCODED_RUSTFLAGS") // remove flags for coverage, incompatible with wasm build
        .output()
        .chain_err(|| "Error while attempting to spawn cargo to compile WASM")?;

    check_cargo_error(command, &command_args, &output)?;

    // TODO this could be removed/reduced when cargo --out-dir option is stable
    // no error occurred, so move the built files to final destination and clean-up
    let mut wasm_filename = implementation_source_path.to_path_buf();
    wasm_filename.set_extension("wasm");

    cargo_target_dir.push(wasm_filename.file_name().ok_or("Could not convert filename to str")?);

    // move compiled wasm output into the destination location
    debug!(
        "\tMoving built wasm file from '{}' to '{}'",
        &cargo_target_dir.display(),
        &wasm_destination.display()
    );
    // avoid rename across possibly different file systems
    fs::copy(&cargo_target_dir, wasm_destination)
        .chain_err(|| format!("Could not copy WASM from '{}' to '{}'",
                              cargo_target_dir.display(),
                              wasm_destination.display()))?;
    fs::remove_file(&cargo_target_dir)
        .chain_err(|| format!("Could not remove_file'{}'",
                              cargo_target_dir.display()))
}

/// Run the cargo build to compile wasm from function source
/// Functions must supply a Cargo.toml file named as function.toml that is used
/// to build the function.
/// If it has already been copied to Cargo.toml for building, then skip that step
/// otherwise copy it and delete it when done, as these files get in the way of
/// publishing the library as a crate
pub fn run(implementation_source_path: &Path, target_dir: PathBuf, wasm_destination: &Path, release_build: bool) -> Result<()> {
    let mut cargo_toml = implementation_source_path.to_path_buf();
    cargo_toml.set_file_name("Cargo.toml");
    let mut function_toml = implementation_source_path.to_path_buf();
    function_toml.set_file_name("function.toml");

    let create_cargo = !cargo_toml.exists();

    if create_cargo {
        fs::copy(function_toml, &cargo_toml)?;
    }

    cargo_test(&cargo_toml.clone())?;

    cargo_build(
        &cargo_toml.clone(),
        target_dir,
        release_build,
        implementation_source_path,
        wasm_destination,
    )?;

    if create_cargo {
        fs::remove_file(cargo_toml)?;
    }

    Ok(())
}

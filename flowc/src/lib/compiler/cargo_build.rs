use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use colored::Colorize;
use log::debug;
use tempdir::TempDir;

use crate::errors::*;

/*
   Check the command Output for an error and print details if it failed
*/
fn check_cargo_error(command: &str, args: Vec<&str>, output: Output) -> Result<()> {
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

fn cargo_test(manifest_path: PathBuf, build_dir: PathBuf) -> Result<()> {
    let command = "cargo";

    debug!("\t Cargo build directory: '{}'", build_dir.display());

    let manifest_arg = format!("--manifest-path={}", manifest_path.display());
    let target_dir_arg = format!("--target-dir={}", build_dir.display());
    let test_args = vec!["+nightly", "test", &manifest_arg, &target_dir_arg];

    println!(
        "   {} {} WASM Project",
        "Testing".green(),
        manifest_path.display()
    );

    debug!("\tRunning command = '{}', args = {:?}", command, test_args);

    let output = Command::new(command)
        .args(&test_args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .chain_err(|| "Error while attempting to spawn cargo to test WASM Project")?;

    check_cargo_error(command, test_args, output)
}

/*
   Run the cargo command that builds the WASM output file
*/
fn cargo_build(
    manifest_path: PathBuf,
    build_dir: &Path,
    release_build: bool,
    implementation_source_path: &Path,
    wasm_destination: &Path,
) -> Result<()> {
    let command = "cargo";
    let manifest = format!("--manifest-path={}", manifest_path.display());
    let target_dir = format!("--target-dir={}", build_dir.display());

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

    let mut command_args = vec!["+nightly", "build"];

    if release_build {
        command_args.push("--release");
    }

    command_args.append(&mut vec![
        "--lib",
        "--target=wasm32-unknown-unknown",
        &manifest,
        &target_dir,
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

    check_cargo_error(command, command_args, output)?;

    // TODO this could be removed/reduced when cargo --out-dir option is stable
    // no error occurred, so move the built files to final destination and clean-up
    let mut wasm_filename = implementation_source_path.to_path_buf();
    wasm_filename.set_extension("wasm");
    let mut wasm_build_location = build_dir.to_path_buf();

    if release_build {
        wasm_build_location.push("wasm32-unknown-unknown/release/");
    } else {
        wasm_build_location.push("wasm32-unknown-unknown/debug/");
    }

    wasm_build_location.push(
        wasm_filename
            .file_name()
            .ok_or("Could not convert filename to str")?,
    );

    // move compiled wasm output into destination location
    debug!(
        "\tMoving built wasm file from '{}' to '{}'",
        &wasm_build_location.display(),
        &wasm_destination.display()
    );
    fs::rename(&wasm_build_location, wasm_destination)
        .chain_err(|| "Could not move WASM to destination")
}

/// Run the cargo build to compile wasm from function source
pub fn run(implementation_source_path: &Path, wasm_destination: &Path, release_build: bool) -> Result<()> {
    let mut manifest_path = implementation_source_path.to_path_buf();
    manifest_path.set_file_name("FlowCargo.toml");

    let mut cargo_toml = manifest_path.clone();
    cargo_toml.set_file_name("Cargo.toml");
    fs::copy(manifest_path, &cargo_toml)?;

    // Create a temp directory for building in. To avoid the corner case where the TempDir
    // maybe on another FS from the destination (preventing renaming) I create it under the
    // destination directory - but it will be cleaned up when `build_dir` goes out of scope
    let build_dir = TempDir::new_in(
        wasm_destination
            .parent()
            .ok_or("Could not create temp dir for WASM building")?,
        "flow",
    )
    .chain_err(|| "Error creating new TempDir for compiling in")?
    .into_path();

    cargo_test(cargo_toml.clone(), build_dir.clone())?;
    cargo_build(
        cargo_toml.clone(),
        &build_dir,
        release_build,
        implementation_source_path,
        wasm_destination,
    )?;

    // clean up temp dir
    fs::remove_dir_all(&build_dir).chain_err(|| {
        format!(
            "Could not remove temporary build directory '{}'",
            build_dir.display()
        )
    })?;

    fs::remove_file(&cargo_toml)
        .chain_err(|| "Could not remove temporary Cargo.toml")?;

    cargo_toml.set_extension("lock");
    let _ = fs::remove_file(cargo_toml);

    Ok(())
}

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
            println!(
                "{}\n{}",
                "Process STDOUT:".green(),
                String::from_utf8_lossy(&output.stdout).green()
            );
            println!(
                "{}\n{}",
                "Process STDERR:".red(),
                String::from_utf8_lossy(&output.stderr).red()
            );
            bail!(
                "cargo exited with status code: {}\nCommand Line: {} {:?}",
                code,
                command,
                args
            )
        }
    }
}

fn cargo_test(manifest_path: PathBuf, build_dir: PathBuf) -> Result<()> {
    let command = "cargo";

    debug!("\t Cargo build directory: '{}'", build_dir.display());

    let manifest_arg = format!("--manifest-path={}", manifest_path.display());
    let target_dir_arg = format!("--target-dir={}", build_dir.display());
    let test_args = vec!["test", &manifest_arg, &target_dir_arg];

    println!(
        "   {} {} WASM Project",
        "Testing".green(),
        manifest_path.display()
    );

    debug!("\tRunning command = '{}', args = {:?}", command, test_args);

    let output = Command::new(&command)
        .env_remove("RUSTFLAGS") // remove flags for coverage, incompatible with wasm build
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

    let command_args = vec![
        "build",
        "--release",
        "--lib",
        "--target=wasm32-unknown-unknown",
        &manifest,
        &target_dir,
    ];

    debug!(
        "\tRunning command = '{}', command_args = {:?}",
        command, command_args
    );

    let output = Command::new(&command)
        .env_remove("RUSTFLAGS") // remove flags for coverage, incompatible with wasm build
        .args(&command_args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .chain_err(|| "Error while attempting to spawn cargo to compile WASM")?;

    check_cargo_error(command, command_args, output)?;

    // TODO this could be removed/reduced when cargo --out-dir option is stable
    // no error occurred, so move the built files to final destination and clean-up
    let mut wasm_filename = implementation_source_path.to_path_buf();
    wasm_filename.set_extension("wasm");
    let mut wasm_build_location = build_dir.to_path_buf();
    wasm_build_location.push("wasm32-unknown-unknown/release/");
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
    fs::rename(&wasm_build_location, &wasm_destination)
        .chain_err(|| "Could not move WASM to destination")
}

/// Run the cargo build to compile wasm from function source
pub fn run(implementation_source_path: &Path, wasm_destination: &Path) -> Result<()> {
    let mut cargo_manifest_path = implementation_source_path.to_path_buf();
    cargo_manifest_path.set_file_name("Cargo.toml");

    // Create a temp directory for building in. Use `new_in` to make sure it is in the same FS as the destination so
    // that fs::rename later works. It will be cleaned-up when `build_dir` goes out of scope.
    let build_dir = TempDir::new_in(wasm_destination.parent().ok_or("Could not get directory of wasm destination to create TempDir in")?, "flow")
        .chain_err(|| "Error creating new TempDir for compiling in")?
        .into_path();

    cargo_test(cargo_manifest_path.clone(), build_dir.clone())?;
    cargo_build(
        cargo_manifest_path,
        &build_dir,
        implementation_source_path,
        wasm_destination,
    )?;

    // clean up temp dir
    fs::remove_dir_all(&build_dir).chain_err(|| {
        format!(
            "Could not remove temporary build directory '{}'",
            build_dir.display()
        )
    })
}

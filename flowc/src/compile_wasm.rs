use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;

use tempdir::TempDir;

use flowclib::model::function::Function;
use provider::args::url_from_string;

use crate::errors::*;

/*
    Compile a function provided in rust to wasm and modify implementation to point to new file
*/
pub fn compile_implementation(function: &mut Function, skip_building: bool) -> Result<PathBuf> {
    let source = function.get_source_url();
    let mut implementation_url = url_from_string(Some(&source)).expect("Could not create a url from source url");
    implementation_url = implementation_url.join(&function.get_implementation()
        .ok_or("No implementation specified")?).map_err(|_| "Could not convert Url")?;

    // TODO what if not a file url? Copy and build locally?

    let implementation_path = implementation_url.to_file_path().expect("Could not convert source url to file path");
    if implementation_path.extension().ok_or("No file extension on source file")?.
        to_str().ok_or("Could not convert file extension to String")? != "rs" {
        bail!("Source file at '{}' does not have a '.rs' extension", implementation_path.display());
    }

    if !implementation_path.exists() {
        bail!("Source file at '{}' does not exist", implementation_path.display());
    }

    // check that a Cargo.toml file exists for compilation
    let mut cargo_path = implementation_path.clone();
    cargo_path.set_file_name("Cargo.toml");
    if !cargo_path.exists() {
        bail!("No Cargo.toml file could be found at '{}'", cargo_path.display());
    }
    info!("Building using rust manifest at '{}'", cargo_path.display());

    let mut wasm_destination = implementation_path.clone();
    wasm_destination.set_extension("wasm");

    // wasm file is out of date if it doesn't exist of timestamp is older than source
    let missing = !wasm_destination.exists();
    let out_of_date = missing || out_of_date(&implementation_path, &wasm_destination)?;

    if missing || out_of_date {
        if skip_building {
            if missing {
                let message = format!("Implementation at '{}' is missing so the flow cannot be executed.\nEither build manually or have 'flowc' build it by not using the '-p' option", wasm_destination.display());
                error!("{}", message);
                bail!(message);
            }
            if out_of_date {
                warn!("Implementation at '{}' is out of date with source at '{}'", wasm_destination.display(), implementation_path.display());
            }
        } else {
            info!("Building wasm '{}' from source '{}'", wasm_destination.display(), implementation_path.display());

            let build_dir = TempDir::new("flow")
                .expect("Error creating new TempDir for compiling in")
                .into_path();

            run_cargo_build(&cargo_path, &build_dir)?;

            // copy compiled wasm output into place where flow's toml file expects it
            let mut wasm_source = build_dir.clone();
            wasm_source.push("wasm32-unknown-unknown/release/");
            wasm_source.push(&wasm_destination.file_name().ok_or("Could not convert filename to str")?);
            info!("Copying built wasm from '{}' to '{}'", &wasm_source.display(), &wasm_destination.display());
            fs::copy(&wasm_source, &wasm_destination).expect("Could not copy wasm file");

            // clean up temp dir
            fs::remove_dir_all(build_dir).expect("Could not remove temporary build directory");
        }
    } else {
        info!("wasm at '{}' is up-to-date with source at '{}', so skipping build",
              wasm_destination.display(), implementation_path.display());
    }

    function.set_implementation(&wasm_destination.to_str().ok_or("Could not convert path to string")?);

    Ok(wasm_destination)
}

/*
    Run the cargo build to compile wasm from function source
*/
fn run_cargo_build(manifest_path: &PathBuf, target_dir: &PathBuf) -> Result<String> {
    info!("Building into temporary directory '{}'", target_dir.display());

    let command = "cargo";
    let mut command_args = vec!("build", "--quiet", "--release", "--lib", "--target=wasm32-unknown-unknown");
    let manifest = format!("--manifest-path={}", &manifest_path.display());
    command_args.push(&manifest);
    let target = format!("--target-dir={}", &target_dir.display());
    command_args.push(&target);

    debug!("Building with command = '{}', command_args = {:?}", command, command_args);

    let output = Command::new(&command).args(command_args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::piped())
        .output().chain_err(|| "Error while attempting to spawn command to compile and run flow")?;

    match output.status.code() {
        Some(0) => Ok("Cargo Build of supplied function to wasm succeeded".to_string()),
        Some(code) => {
            error!("Process STDERR:\n{}", String::from_utf8_lossy(&output.stderr));
            bail!("Exited with status code: {}", code)
        }
        None => Ok("No return code - ignoring".to_string())
    }
}

/*
    Determine if one file that is derived from another source is out of date (source is newer
    that derived)
    Returns:
        true - source file has been modified since the derived file was last modified
        false - source has not been modified since derived file was last modified
*/
fn out_of_date(source: &PathBuf, derived: &PathBuf) -> Result<bool> {
    let source_last_modified = fs::metadata(source)
        .expect("Could not get file metadata")
        .modified().expect("Could not get modified time from file metadata");
    let derived_last_modified = fs::metadata(derived)
        .expect("Could not get file metadata")
        .modified().expect("Could not get modified time from file metadata");
    Ok(source_last_modified > derived_last_modified)
}
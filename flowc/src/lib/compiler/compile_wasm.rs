use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::process::Stdio;

use log::{debug, error, info, warn};
use tempdir::TempDir;
use url::Url;

use flowcore::url_helper::url_from_string;

use crate::errors::*;
use crate::generator::generate::GenerationTables;
use crate::model::function::Function;

/// For any function that provides an implementation - compile the source to wasm and modify the
/// implementation to indicate it is the wasm file
pub fn compile_supplied_implementations(
    tables: &mut GenerationTables,
    skip_building: bool,
) -> Result<String> {
    for function in &mut tables.functions {
        if function.get_lib_reference().is_none() {
            compile_implementation(function, skip_building)?;
        }
    }

    Ok("All supplied implementations compiled successfully".into())
}

/// Compile a function provided in rust to wasm and modify implementation to point to new file
/// Checks the timestamp of source and wasm files and only recompiles if wasm file is out of date
pub fn compile_implementation(
    function: &mut Function,
    skip_building: bool,
) -> Result<(PathBuf, bool)> {
    let mut built = false;

    let (implementation_path, wasm_destination) = get_paths(function)?;

    let (missing, out_of_date) = out_of_date(&implementation_path, &wasm_destination)?;

    if missing || out_of_date {
        if skip_building {
            if missing {
                let message = format!("Implementation at '{}' is missing so the flow cannot be executed.\nEither build manually or have 'flowc' build it by not using the '-p' option", wasm_destination.display());
                error!("{}", message);
                bail!(message);
            }
            if out_of_date {
                warn!(
                    "Implementation at '{}' is out of date with source at '{}'",
                    wasm_destination.display(),
                    implementation_path.display()
                );
            }
        } else {
            debug!(
                "Building wasm '{}' from source '{}'",
                wasm_destination.display(),
                implementation_path.display()
            );

            let build_dir = TempDir::new("flow")
                .chain_err(|| "Error creating new TempDir for compiling in")?
                .into_path();

            // check that a Cargo.toml file exists for compilation
            let mut flow_manifest_path = implementation_path.clone();
            flow_manifest_path.set_file_name("flow.toml");
            if !flow_manifest_path.exists() {
                bail!(
                    "No flow.toml file could be found at '{}'",
                    flow_manifest_path.display()
                );
            }

            let mut cargo_manifest_path = flow_manifest_path.clone();
            cargo_manifest_path.set_file_name("Cargo.toml");

            // Copy 'flow.toml' to 'Cargo.toml' so that 'cargo' will compile it
            fs::copy(&flow_manifest_path, &cargo_manifest_path).map_err(|e| {
                format!(
                    "Error while trying to copy '{}' to '{}'\n{}",
                    flow_manifest_path.display(),
                    cargo_manifest_path.display(),
                    e.to_string()
                )
            })?;

            info!("Compiling to WASM '{}'", implementation_path.display());
            run_cargo_build(&cargo_manifest_path, &build_dir)?;

            // copy compiled wasm output into place where flow's toml file expects it
            let mut wasm_source = build_dir.clone();
            wasm_source.push("wasm32-unknown-unknown/debug/");
            wasm_source.push(
                &wasm_destination
                    .file_name()
                    .ok_or("Could not convert filename to str")?,
            );
            let msg = format!(
                "Copying built wasm from '{}' to '{}'",
                &wasm_source.display(),
                &wasm_destination.display()
            );
            fs::copy(&wasm_source, &wasm_destination).chain_err(|| msg)?;

            // clean up temp dir
            fs::remove_dir_all(&build_dir).chain_err(|| {
                format!(
                    "Could not remove temporary build directory '{}'",
                    build_dir.display()
                )
            })?;

            // remove the file we copied
            fs::remove_file(&cargo_manifest_path).chain_err(|| {
                format!(
                    "Could not remove copied file '{}'",
                    cargo_manifest_path.display()
                )
            })?;

            built = true;
        }
    } else {
        debug!(
            "wasm at '{}' is up-to-date with source at '{}', so skipping build",
            wasm_destination.display(),
            implementation_path.display()
        );
    }

    function.set_implementation(
        &wasm_destination
            .to_str()
            .ok_or("Could not convert path to string")?,
    );

    Ok((wasm_destination, built))
}

/*
    Run the cargo build to compile wasm from function source
*/
fn run_cargo_build(manifest_path: &Path, target_dir: &Path) -> Result<String> {
    debug!(
        "Building into temporary directory '{}'",
        target_dir.display()
    );

    let command = "cargo";
    let mut command_args = vec![
        "build",
        "--quiet",
        // "--verbose",
        "--lib",
        "--target=wasm32-unknown-unknown",
    ];
    let manifest = format!("--manifest-path={}", &manifest_path.display());
    command_args.push(&manifest);
    let target = format!("--target-dir={}", &target_dir.display());
    command_args.push(&target);

    debug!(
        "Building with command = '{}', command_args = {:?}",
        command, command_args
    );

    let output = Command::new(&command)
        .env_remove("RUSTFLAGS") // remove flags for coverage, incompatible with wasm build
        .args(&command_args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .chain_err(|| "Error while attempting to spawn command to compile and run flow")?;

    match output.status.code() {
        Some(0) => Ok("Cargo Build of supplied function to wasm succeeded".to_string()),
        Some(code) => {
            error!(
                "Process STDOUT:\n{}",
                String::from_utf8_lossy(&output.stdout)
            );
            error!(
                "Process STDERR:\n{}",
                String::from_utf8_lossy(&output.stderr)
            );
            bail!(
                "cargo exited with status code: {}\nCommand Line: {} {:?}",
                code,
                command,
                command_args
            )
        }
        None => Ok("No return code - ignoring".to_string()),
    }
}

/*
   Calculate the paths to the source file of the implementation of the function to be compiled
   and where to output the compiled wasm
*/
fn get_paths(function: &Function) -> Result<(PathBuf, PathBuf)> {
    let cwd = env::current_dir().chain_err(|| "Could not get current working directory value")?;
    let cwd_url = Url::from_directory_path(cwd)
        .map_err(|_| "Could not form a Url for the current working directory")?;

    let function_source_url = url_from_string(&cwd_url, Some(&function.get_source_url()))
        .chain_err(|| "Could not create a url from source url")?;
    let implementation_source_url = function_source_url
        .join(&function.get_implementation())
        .map_err(|_| "Could not convert Url")?;

    let implementation_source_path = implementation_source_url
        .to_file_path()
        .map_err(|_| "Could not convert source url to file path")?;
    if implementation_source_path
        .extension()
        .ok_or("No file extension on source file")?
        .to_str()
        .ok_or("Could not convert file extension to String")?
        != "rs"
    {
        bail!(
            "Source file at '{}' does not have a '.rs' extension",
            implementation_source_path.display()
        );
    }

    if !implementation_source_path.exists() {
        bail!(
            "Source file at '{}' does not exist",
            implementation_source_path.display()
        );
    }

    let mut implementation_wasm_path = implementation_source_path.clone();
    implementation_wasm_path.set_extension("wasm");

    Ok((implementation_source_path, implementation_wasm_path))
}

/*
    Determine if one file that is derived from another source is missing and if not missing
    if it is out of date (source is newer that derived)
    Returns: (out_of_date, missing)
    out_of_date
        true - source file has been modified since the derived file was last modified or is missing
        false - source has not been modified since derived file was last modified
    missing
        true - the derived file does no exist
        false - the derived file does exist
*/
fn out_of_date(source: &Path, derived: &Path) -> Result<(bool, bool)> {
    let source_last_modified = fs::metadata(source)
        .chain_err(|| format!("Could not get metadata for file: '{}'", source.display()))?
        .modified()
        .chain_err(|| "Could not get modified time from file metadata")?;

    if derived.exists() {
        let derived_last_modified = fs::metadata(derived)
            .chain_err(|| format!("Could not get metadata for file: '{}'", derived.display()))?
            .modified()
            .chain_err(|| "Could not get modified time from file metadata")?;
        Ok(((source_last_modified > derived_last_modified), false))
    } else {
        Ok((true, true))
    }
}

#[cfg(test)]
mod test {
    use std::fs::{remove_file, write};
    use std::path::Path;
    use std::time::Duration;

    use flowcore::output_connection::OutputConnection;

    use crate::model::function::Function;
    use crate::model::io::IO;
    use crate::model::route::Route;

    use super::get_paths;
    use super::out_of_date;

    #[test]
    fn out_of_date_test() {
        let output_dir = tempdir::TempDir::new("flow")
            .unwrap_or_else(|_| panic!("Could not create TempDir during testing"))
            .into_path();

        // make older file
        let older = output_dir.join("older");
        let derived = older.clone();
        write(&older, "older").unwrap_or_else(|_| {
            panic!("Could not write to file {} during testing", older.display())
        });

        std::thread::sleep(Duration::from_secs(1));

        // make second/newer file
        let newer = output_dir.join("newer");
        let source = newer.clone();
        write(&newer, "newer").unwrap_or_else(|_| {
            panic!("Could not write to file {} during testing", newer.display())
        });

        assert!(
            out_of_date(&source, &derived)
                .unwrap_or_else(|_| panic!("Error in 'out__of_date'"))
                .0
        );
    }

    #[test]
    fn not_out_of_date_test() {
        let output_dir = tempdir::TempDir::new("flow")
            .unwrap_or_else(|_| panic!("Could not create TempDir during testing"))
            .into_path();

        // make older file
        let older = output_dir.join("older");
        let source = older.clone();
        write(&older, "older").unwrap_or_else(|_| {
            panic!("Could not write to file {} during testing", older.display())
        });

        // make second/newer file
        let newer = output_dir.join("newer");
        let derived = newer.clone();
        write(&newer, "newer").unwrap_or_else(|_| {
            panic!("Could not write to file {} during testing", newer.display())
        });

        assert_eq!(
            out_of_date(&source, &derived)
                .unwrap_or_else(|_| panic!("Error in 'out__of_date'"))
                .0,
            false
        );
    }

    #[test]
    fn out_of_date_missing_test() {
        let output_dir = tempdir::TempDir::new("flow")
            .unwrap_or_else(|_| panic!("Could not create TempDir during testing"))
            .into_path();

        // make older file
        let older = output_dir.join("older");
        let source = older.clone();
        write(&older, "older").unwrap_or_else(|_| {
            panic!("Could not write to file {} during testing", older.display())
        });

        // make second/newer file
        let newer = output_dir.join("newer");
        write(&newer, "newer").unwrap_or_else(|_| {
            panic!("Could not write to file {} during testing", newer.display())
        });

        let derived = newer.clone();
        remove_file(newer).unwrap_or_else(|_| panic!("Error in 'remove_file' during testing"));

        assert!(
            out_of_date(&source, &derived)
                .unwrap_or_else(|_| panic!("Error in 'out__of_date'"))
                .1
        );
    }

    fn test_function() -> Function {
        Function::new(
            "Stdout".into(),
            false,
            "stdout.rs".to_string(),
            "print".into(),
            vec![],
            vec![IO::new("String", Route::default())],
            &format!(
                "{}/{}",
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .parent()
                    .expect("Error getting Manifest Dir")
                    .display()
                    .to_string(),
                "flowr/src/lib/flowruntime/stdio/stdout"
            ),
            Route::from("/flow0/stdout"),
            Some("flowruntime/stdio/stdout".to_string()),
            vec![OutputConnection::new(
                "".to_string(),
                1,
                0,
                0,
                0,
                false,
                None,
                #[cfg(feature = "debugger")]
                String::default(),
            )],
            0,
            0,
        )
    }

    #[test]
    fn paths_test() {
        let function = test_function();

        let (impl_source_path, impl_wasm_path) =
            get_paths(&function).expect("Error in 'get_paths'");

        assert_eq!(
            format!(
                "{}/{}",
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .parent()
                    .expect("Error getting Manifest Dir")
                    .display()
                    .to_string(),
                "flowr/src/lib/flowruntime/stdio/stdout.rs"
            ),
            impl_source_path
                .to_str()
                .expect("Error converting path to str")
        );
        assert_eq!(
            format!(
                "{}/{}",
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .parent()
                    .expect("Error getting Manifest Dir")
                    .display()
                    .to_string(),
                "flowr/src/lib/flowruntime/stdio/stdout.wasm"
            ),
            impl_wasm_path
                .to_str()
                .expect("Error converting path to str")
        );
    }
}

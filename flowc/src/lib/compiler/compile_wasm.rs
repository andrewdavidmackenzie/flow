#[cfg(feature = "debugger")]
use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::process::Stdio;

use colored::Colorize;
use log::{debug, error, info};
use simpath::{FileType, FoundType, Simpath};
use tempdir::TempDir;
use url::Url;

use flowcore::url_helper::url_from_string;

use crate::errors::*;
use crate::model::function::Function;

/// Compile a function provided in rust to wasm and modify implementation to point to new file
/// Checks the timestamp of source and wasm files and only recompiles if wasm file is out of date
pub fn compile_implementation(
    function: &mut Function,
    skip_building: bool,
    #[cfg(feature = "debugger")] source_urls: &mut HashSet<(Url, Url)>,
) -> Result<(PathBuf, bool)> {
    let mut built = false;

    let (implementation_path, wasm_destination) = get_paths(function)?;
    #[cfg(feature = "debugger")]
    source_urls.insert((
        Url::from_file_path(&implementation_path)
            .map_err(|_| "Could not create Url from file path")?,
        Url::from_file_path(&wasm_destination)
            .map_err(|_| "Could not create Url from file path")?,
    ));

    let (missing, out_of_date) = out_of_date(&implementation_path, &wasm_destination)?;

    if missing || out_of_date {
        if skip_building {
            if missing {
                let message = format!("Implementation at '{}' is missing so the flow cannot be executed.\nEither build manually or have 'flowc' build it by not using the '-p' option", wasm_destination.display());
                error!("{}", message);
                bail!(message);
            }
            if out_of_date {
                info!(
                    "Implementation at '{}' is out of date with source at '{}'",
                    wasm_destination.display(),
                    implementation_path.display()
                );
            }
        } else {
            run_cargo_build(&implementation_path, &wasm_destination)?;

            built = true;
        }
    } else {
        debug!(
            "wasm at '{}' is up-to-date with source at '{}'",
            wasm_destination.display(),
            implementation_path.display()
        );
    }

    function.set_implementation(
        wasm_destination
            .to_str()
            .ok_or("Could not convert path to string")?,
    );

    Ok((wasm_destination, built))
}

/*
   Try and run a command that may or may not be installed in the system thus:
   - create a temporary directory where the output file will be created
   - run the command: $command $wasm_path $args $temp_file
   - return the path to the $temp_file
*/
fn run_optional_command(wasm_path: &Path, command: &str, mut args: Vec<String>) -> Result<()> {
    if let Ok(FoundType::File(command_path)) =
        Simpath::new("PATH").find_type(command, FileType::File)
    {
        let tmp_dir = TempDir::new("wasm-opt")?;
        let temp_file_path = tmp_dir
            .path()
            .join(wasm_path.file_name().ok_or("Could not get wasm filename")?);
        let mut command = Command::new(&command_path);
        let mut command_args = vec![wasm_path.to_string_lossy().to_string()];
        if !args.is_empty() {
            command_args.append(&mut args);
        }
        command_args.append(&mut vec![temp_file_path.to_string_lossy().to_string()]);
        let child = command
            .args(command_args)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()?;

        let output = child.wait_with_output()?;

        match output.status.code() {
            Some(0) | None => fs::rename(&temp_file_path, &wasm_path)?,
            Some(_) => bail!(format!(
                "{} exited with non-zero status code",
                command_path.to_string_lossy().to_string()
            )),
        }

        // remove the temp dir
        fs::remove_dir_all(&tmp_dir)?;
    }

    Ok(())
}

/*
   Optimize a wasm file's size using equivalent of
    wasm-gc $(file) -o $(file).gc && \
    mv $(file).gc $(file) && \
    wasm-snip $(file) -o $(file).snipped && \
    mv $(file).snipped $(file) && \
    wasm-gc $(file) -o $(file).gc && \
    mv $(file).gc $(file) && \
    wasm-opt $(file) -O4 --dce -o $(file).opt && \
    mv $(file).opt $(file)
*/
fn optimize_wasm_file_size(wasm_path: &Path) -> Result<()> {
    run_optional_command(wasm_path, "wasm-gc", vec!["-o".into()])?;
    run_optional_command(wasm_path, "wasm-snip", vec!["-o".into()])?;
    run_optional_command(wasm_path, "wasm-gc", vec!["-o".into()])?;
    run_optional_command(
        wasm_path,
        "wasm-opt",
        vec!["-O4".into(), "--dce".into(), "-o".into()],
    )
}

fn create_cargo_project(implementation_path: &Path) -> Result<PathBuf> {
    let mut manifest_path = implementation_path.to_path_buf();

    // check that a flow.toml file exists for compilation in the implementation directory
    manifest_path.set_file_name("flow.toml");
    if !manifest_path.exists() {
        bail!(
            "No flow.toml file could be found at '{}'",
            manifest_path.display()
        );
    }

    let mut cargo_manifest_path = manifest_path.clone();
    cargo_manifest_path.set_file_name("Cargo.toml");

    // Copy 'flow.toml' to 'Cargo.toml' so that 'cargo' will compile it
    info!(
        "Copying {} to {}",
        manifest_path.display(),
        cargo_manifest_path.display()
    );

    fs::copy(&manifest_path, &cargo_manifest_path)?;

    Ok(cargo_manifest_path)
}

/*
    Run the cargo build to compile wasm from function source
*/
fn run_cargo_build(implementation_path: &Path, wasm_destination: &Path) -> Result<()> {
    debug!(
        "Building wasm '{}' from source '{}'",
        wasm_destination.display(),
        implementation_path.display()
    );

    let cargo_manifest_path = create_cargo_project(implementation_path)?;

    println!(
        "   {} {} to WASM",
        "Compiling".green(),
        implementation_path.display()
    );

    let build_dir = TempDir::new("flow")
        .chain_err(|| "Error creating new TempDir for compiling in")?
        .into_path();

    debug!("Building into directory '{}'", build_dir.display());

    let command = "cargo";
    let mut command_args = vec![
        "build",
        "--quiet",
        "--release",
        "--lib",
        "--target=wasm32-unknown-unknown",
    ];
    let manifest = format!("--manifest-path={}", &cargo_manifest_path.display());
    command_args.push(&manifest);
    let target_dir = format!("--target-dir={}", &build_dir.display());
    command_args.push(&target_dir);

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
        .chain_err(|| "Error while attempting to spawn cargo to compile WASM")?;

    // remove the Cargo.toml file we created in the source directory
    fs::remove_file(&cargo_manifest_path).chain_err(|| {
        format!(
            "Could not remove copied file '{}'",
            cargo_manifest_path.display()
        )
    })?;

    match output.status.code() {
        Some(0) | None => {
            let mut wasm_filename = implementation_path.to_path_buf();
            wasm_filename.set_extension("wasm");
            let mut wasm_build_location = build_dir.clone();
            wasm_build_location.push("wasm32-unknown-unknown/release/");
            wasm_build_location.push(
                wasm_filename
                    .file_name()
                    .ok_or("Could not convert filename to str")?,
            );
            optimize_wasm_file_size(&wasm_build_location)?;

            // copy compiled wasm output into place where flow's toml file expects it
            fs::copy(&wasm_build_location, &wasm_destination)
                .chain_err(|| "Could not copy WASM to destination")?;

            // clean up temp dir
            fs::remove_dir_all(&build_dir).chain_err(|| {
                format!(
                    "Could not remove temporary build directory '{}'",
                    build_dir.display()
                )
            })?;

            Ok(())
        }
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

    let function_source_url = url_from_string(&cwd_url, Some(function.get_source_url()))
        .chain_err(|| "Could not create a url from source url")?;
    let implementation_source_url = function_source_url
        .join(function.get_implementation())
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
    use std::fs::{remove_file, write, File};
    use std::path::Path;
    use std::time::Duration;

    use tempdir::TempDir;

    use flowcore::output_connection::{OutputConnection, Source};

    use crate::model::function::Function;
    use crate::model::io::IO;
    use crate::model::route::Route;

    use super::out_of_date;
    use super::{get_paths, run_optional_command};

    #[test]
    fn test_run_optional_non_existent() {
        let _ = run_optional_command(Path::new("/tmp"), "foo", vec!["bar".into()]);
    }

    #[test]
    fn test_run_optional_exists() {
        let temp_dir = TempDir::new("flow-tests").expect("Could not get temp dir");
        let temp_file_path = temp_dir.path().join("from.test");
        File::create(&temp_file_path).expect("Could not create test file");
        let _ = run_optional_command(temp_file_path.as_path(), "cp", vec![]);
        assert!(temp_file_path.exists());
    }

    #[test]
    fn test_run_optional_exists_fail() {
        let temp_dir = TempDir::new("flow-tests").expect("Could not get temp dir");
        let temp_file_path = temp_dir.path().join("from.test");
        File::create(&temp_file_path).expect("Could not create test file");
        let _ = run_optional_command(
            temp_file_path.as_path(),
            "cp",
            vec!["--no-such-flag".into()],
        );
        assert!(temp_file_path.exists());
    }

    #[test]
    fn copy_valid_flow_toml() {
        let temp_dir = TempDir::new("flow-tests").expect("Could not get temp dir");
        let mut temp_file_path = temp_dir.path().join("flow.toml");
        File::create(&temp_file_path).expect("Could not create test file");
        let _ = super::create_cargo_project(temp_file_path.as_path());
        temp_file_path.set_file_name("Cargo.toml");
        assert!(temp_file_path.exists());
    }

    #[test]
    fn copy_invalid_flow_toml() {
        let temp_dir = TempDir::new("flow-tests").expect("Could not get temp dir");
        let temp_file_path = temp_dir.path().join("flow.toml");
        assert!(super::create_cargo_project(temp_file_path.as_path()).is_err());
    }

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
            .expect("Could not create TempDir during testing")
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

        assert!(
            !out_of_date(&source, &derived)
                .unwrap_or_else(|_| panic!("Error in 'out__of_date'"))
                .0
        );
    }

    #[test]
    fn out_of_date_missing_test() {
        let output_dir = tempdir::TempDir::new("flow")
            .expect("Could not create TempDir during testing")
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
                Source::default(),
                1,
                0,
                0,
                0,
                false,
                String::default(),
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

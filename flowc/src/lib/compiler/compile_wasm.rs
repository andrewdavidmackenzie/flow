#[cfg(feature = "debugger")]
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::process::Stdio;

use log::{debug, info, warn};
use simpath::{FileType, FoundType, Simpath};
use tempdir::TempDir;
#[cfg(feature = "debugger")]
use url::Url;

use flowcore::model::function_definition::FunctionDefinition;

use crate::compiler::cargo_build;
use crate::errors::*;

/// Compile a function's implementation to wasm and modify implementation to point to the wasm file
/// Checks the timestamps of the source and wasm files and only recompiles if wasm file is out of date
pub fn compile_implementation(
    cargo_target_dir: PathBuf, // where the binary will be built by cargo
    wasm_destination: &Path,
    implementation_source_path: PathBuf,
    function: &mut FunctionDefinition,
    native_only: bool,
    optimize: bool,
    #[cfg(feature = "debugger")] source_urls: &mut BTreeSet<(Url, Url)>,
) -> Result<bool> {
    let mut built = false;

    #[cfg(feature = "debugger")]
    source_urls.insert((
        Url::from_file_path(&implementation_source_path).map_err(|_| "Could not create Url from file path")?,
        Url::from_file_path(wasm_destination)
            .map_err(|_| "Could not create Url from file path")?,
    ));

    let (missing, out_of_date) = out_of_date(&implementation_source_path, wasm_destination)?;

    if missing || out_of_date {
        if native_only {
            if missing {
                warn!("Implementation '{}' is missing and you have \
                selected to skip building, so flows relaying on this implementation will not \
                execute correctly.\nYou can build it using 'flowc', using the '-p' option",
                                      wasm_destination.display());
            }
            if out_of_date {
                info!(
                    "Implementation '{}' is out of date with source at '{}'",
                    wasm_destination.display(),
                    implementation_source_path.display()
                );
            }
        } else {
            match function.build_type.as_str() {
                "rust" => {
                    cargo_build::run(&implementation_source_path, cargo_target_dir,
                                     wasm_destination, optimize)?
                },
                _ => bail!(
                    "Unknown build type '{}' for function at '{}'",
                    implementation_source_path.display(),
                    function.build_type),
            }

            if optimize {
                optimize_wasm_file_size(wasm_destination)?;
            }
            built = true;
        }
    } else {
        debug!(
            "wasm at '{}' is up-to-date with source at '{}'",
            wasm_destination.display(),
            implementation_source_path.display()
        );
    }

    function.set_implementation(
        wasm_destination
            .to_str()
            .ok_or("Could not convert path to string")?,
    );

    Ok(built)
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
        // Create a temp directory for building in. Use `new_in` to make sure it is in the same FS as the destination so
        // that fs::rename later works. It will be cleaned-up when `build_dir` goes out of scope.
        let tmp_dir = TempDir::new_in(
            wasm_path
                .parent()
                .ok_or("Could not get destination directory to create TempDir in")?,
            "wasm-opt",
        )?;
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
            .stderr(Stdio::inherit());

        let output = child.output()?;

        match output.status.code() {
            Some(0) | None => fs::rename(&temp_file_path, wasm_path)?,
            Some(_) => bail!(format!(
                "{} exited with non-zero status code",
                command_path.to_string_lossy()
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
        .modified()?;

    if derived.exists() {
        let derived_last_modified = fs::metadata(derived)
            .chain_err(|| format!("Could not get metadata for file: '{}'", derived.display()))?
            .modified()?;
        Ok(((source_last_modified > derived_last_modified), false))
    } else {
        Ok((true, true))
    }
}

#[cfg(test)]
mod test {
    #[cfg(feature = "debugger")]
    use std::collections::BTreeSet;
    use std::env;
    use std::fs::{File, remove_file, write};
    use std::path::Path;
    use std::time::Duration;

    use tempdir::TempDir;
    use url::Url;

    use flowcore::model::datatype::STRING_TYPE;
    use flowcore::model::function_definition::FunctionDefinition;
    use flowcore::model::io::IO;
    use flowcore::model::output_connection::{OutputConnection, Source};
    use flowcore::model::route::Route;

    use crate::compiler::compile;

    use super::out_of_date;
    use super::run_optional_command;

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
    fn out_of_date_test() {
        let output_dir = TempDir::new("flow")
            .expect("Could not create TempDir during testing")
            .into_path();

        // make older file
        let older = output_dir.join("older");
        let derived = older.clone();
        write(&older, "older").expect("Could not write to file during testing");

        std::thread::sleep(Duration::from_secs(1));

        // make second/newer file
        let newer = output_dir.join("newer");
        let source = newer.clone();
        write(&newer, "newer").expect("Could not write to file during testing");

        assert!(
            out_of_date(&source, &derived)
                .expect("Error in 'out__of_date'")
                .0
        );
    }

    #[test]
    fn not_out_of_date_test() {
        let output_dir = TempDir::new("flow")
            .expect("Could not create TempDir during testing")
            .into_path();

        // make older file
        let older = output_dir.join("older");
        let source = older.clone();
        write(&older, "older").expect("Could not write to file {} during testing");

        // make second/newer file
        let newer = output_dir.join("newer");
        let derived = newer.clone();
        write(&newer, "newer").expect("Could not write to file {} during testing");

        assert!(
            !out_of_date(&source, &derived)
                .expect("Error in 'out_of_date'")
                .0
        );
    }

    #[test]
    fn out_of_date_missing_test() {
        let output_dir = TempDir::new("flow")
            .expect("Could not create TempDir during testing")
            .into_path();

        // make older file
        let older = output_dir.join("older");
        let source = older.clone();
        write(&older, "older").expect("Could not write to file {} during testing");

        // make second/newer file
        let newer = output_dir.join("newer");
        write(&newer, "newer").expect("Could not write to file {} during testing");

        let derived = newer.clone();
        remove_file(newer).unwrap_or_else(|_| panic!("Error in 'remove_file' during testing"));

        assert!(
            out_of_date(&source, &derived)
                .expect("Error in 'out__of_date'")
                .1
        );
    }

    fn test_function() -> FunctionDefinition {
        FunctionDefinition::new(
            "Stdout".into(),
            false,
            "test.rs".to_string(),
            "print".into(),
            vec![IO::new(vec!(STRING_TYPE.into()), Route::default())],
            vec![IO::new(vec!(STRING_TYPE.into()), Route::default())],
            Url::parse(&format!(
                "file://{}/{}",
                env!("CARGO_MANIFEST_DIR"),
                "tests/test-functions/test/test"
            ))
            .expect("Could not create source Url"),
            Route::from("/flow0/stdout"),
            Some(Url::parse("lib::/tests/test-functions/test/test")
                .expect("Could not parse Url")),
            None,
            vec![OutputConnection::new(
                Source::default(),
                1,
                0,
                0,
                String::default(),
                #[cfg(feature = "debugger")]
                String::default(),
            )],
            0,
            0,
        )
    }

    #[test]
    fn test_compile_implementation_skip_missing() {
        let mut function = test_function();

        let wasm_output_dir = TempDir::new("flow")
            .expect("Could not create TempDir during testing")
            .into_path();
        let expected_output_wasm = wasm_output_dir.join("test.wasm");
        let _ = remove_file(&expected_output_wasm);

        #[cfg(feature = "debugger")]
        let mut source_urls = BTreeSet::<(Url, Url)>::new();

        let (implementation_source_path, wasm_destination) = compile::get_paths(&wasm_output_dir, &function)
            .expect("Could not get paths for compiling");
        assert_eq!(wasm_destination, expected_output_wasm);

        let mut cargo_target_dir = implementation_source_path.parent()
            .ok_or("Could not get directory where Cargo.toml resides")
            .expect("Could not get source directory").to_path_buf();
        cargo_target_dir.push("target");

        let built = super::compile_implementation(
            cargo_target_dir,
            &wasm_output_dir,
            implementation_source_path,
            &mut function,
            true,
            false,
            #[cfg(feature = "debugger")]
            &mut source_urls,
        )
        .expect("compile_implementation() failed");

        assert!(!built);
    }

    #[test]
    fn test_compile_implementation_not_needed() {
        let mut function = test_function();

        let wasm_output_dir = TempDir::new("flow")
            .expect("Could not create TempDir during testing")
            .into_path();
        let expected_output_wasm = wasm_output_dir.join("test.wasm");

        let _ = remove_file(&expected_output_wasm);
        write(&expected_output_wasm, b"file touched during testing")
            .expect("Could not write to file during testing");
        #[cfg(feature = "debugger")]
        let mut source_urls = BTreeSet::<(Url, Url)>::new();

        let (implementation_source_path, wasm_destination) = compile::get_paths(&wasm_output_dir, &function)
            .expect("Could not get paths for compiling");
        assert_eq!(wasm_destination, expected_output_wasm);

        let mut cargo_target_dir = implementation_source_path.parent()
            .ok_or("Could not get directory where Cargo.toml resides")
            .expect("Could not get source directory").to_path_buf();
        cargo_target_dir.push("target");

        let built = super::compile_implementation(
            cargo_target_dir,
            &wasm_output_dir,
            implementation_source_path,
            &mut function,
            false,
            false,
            #[cfg(feature = "debugger")]
            &mut source_urls,
        )
        .expect("compile_implementation() failed");

        assert!(!built); // destination newer than source so should not have been built
    }

    #[test]
    fn test_compile_implementation_skip() {
        let mut function = test_function();

        let wasm_output_dir = TempDir::new("flow")
            .expect("Could not create TempDir during testing")
            .into_path();
        let expected_output_wasm = wasm_output_dir.join("test.wasm");

        #[cfg(feature = "debugger")]
            let mut source_urls = BTreeSet::<(Url, Url)>::new();

        let (implementation_source_path, wasm_destination) = compile::get_paths(&wasm_output_dir, &function)
            .expect("Could not get paths for compiling");
        assert_eq!(expected_output_wasm, wasm_destination);
        let mut cargo_target_dir = implementation_source_path.parent()
            .ok_or("Could not get directory where Cargo.toml resides")
            .expect("Could not get source directory").to_path_buf();
        cargo_target_dir.push("target");

        let built = super::compile_implementation(
            cargo_target_dir,
            &wasm_output_dir,
            implementation_source_path,
            &mut function,
            true,
            false,
            #[cfg(feature = "debugger")]
                &mut source_urls,
        ).expect("compile_implementation() failed");

        assert!(!built);
    }

    #[test]
    fn test_compile_implementation_invalid_paths() {
        let mut function = test_function();
        function.set_source("does_not_exist");

        let wasm_output_dir = TempDir::new("flow")
            .expect("Could not create TempDir during testing")
            .into_path();

        #[cfg(feature = "debugger")]
        let mut source_urls = BTreeSet::<(Url, Url)>::new();

        let (implementation_source_path, _wasm_destination) = compile::get_paths(&wasm_output_dir, &function)
            .expect("Could not get paths for compiling");
        let mut cargo_target_dir = implementation_source_path.parent()
            .ok_or("Could not get directory where Cargo.toml resides")
            .expect("Could not get source directory").to_path_buf();
        cargo_target_dir.push("target");

        assert!(super::compile_implementation(
            cargo_target_dir,
            &wasm_output_dir,
            implementation_source_path,
            &mut function,
            true,
            false,
            #[cfg(feature = "debugger")]
                &mut source_urls,
        )
            .is_err());
    }

    #[test]
    fn test_compile_implementation() {
        let mut function = test_function();
        function.build_type = "rust".into();

        let wasm_output_dir = TempDir::new("flow")
            .expect("Could not create TempDir during testing")
            .into_path();
        let expected_output_wasm = wasm_output_dir.join("test.wasm");
        let _ = remove_file(&expected_output_wasm);

        #[cfg(feature = "debugger")]
            let mut source_urls = BTreeSet::<(Url, Url)>::new();

        let (implementation_source_path, wasm_destination) = compile::get_paths(&wasm_output_dir, &function)
            .expect("Could not get paths for compiling");
        assert_eq!(wasm_destination, expected_output_wasm);

        let mut cargo_target_dir = implementation_source_path.parent()
            .ok_or("Could not get directory where Cargo.toml resides")
            .expect("Could not get source directory").to_path_buf();
        cargo_target_dir.push("target/wasm32-unknown-unknown/debug");

        let built = super::compile_implementation(
            cargo_target_dir,
            &wasm_destination,
            implementation_source_path,
            &mut function,
            false,
            false,
            #[cfg(feature = "debugger")]
                &mut source_urls,
        )
            .expect("compile_implementation() failed");

        assert!(built);
    }
}

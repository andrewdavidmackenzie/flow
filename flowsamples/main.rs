//! `flowsamples` is a set of sample flows, used to test and demonstrate flow, and different
//! semantics and characteristics of flows that can be written.
//!
//! Each subdirectory holds a self-contained flow sample, with flow definition, docs etc and
//! some of them provide their own function implementations.
//!
//! At the top-level there is a `build.rs` build script that iterates over all the sub-folders
//! found and compiles the flow. That will include flow compilation to produce a `manifest.json`
//! flow manifest. If any flow has provided function implementations then they will be compiled
//! to WASM files. When the build has completed, all samples should be ready to be ran, using
//! the `flowr` flow runner.
//!
//! See [main] for details on running the flow samples yourself directly.
//!
//! If `cargo test` is run on this crate, then all the samples will be ran using the provided
//! test input and test args (see names of test files below) and the Stdout and File
//! output will be compared to a set of predefined Stdout and File output to determine if
//! the sample ran correctly. No Stderr output is expected so if any is detected the sample is
//! deemed to have not ran correctly.

use std::{env, fs, io};
use std::fs::File;
use std::io::{BufRead, BufReader, ErrorKind};
use std::path::Path;
use std::process::{Command, Stdio};

/// Name of file where any Stdout will be written while executing a flowsample in test mode
const TEST_STDOUT_FILENAME: &str = "test.stdout";
#[cfg(test)]
/// Name of file where the Stdout is defined
const EXPECTED_STDOUT_FILENAME : &str = "expected.stdout";

/// Name of file where any Stdin will be read from while executing a flowsample in test mode
const TEST_STDIN_FILENAME : &str = "test.stdin";

/// Name of file where any Stderr will be written from while executing a flowsample in test mode
const TEST_STDERR_FILENAME : &str = "test.stderr";

/// Name of file used for file output of a sample
const TEST_FILE_FILENAME: &str = "test.file";
#[cfg(test)]
/// Name of file where expected file output is defined
const EXPECTED_FILE_FILENAME : &str = "expected.file";

/// Name of file where flow arguments for a flow sample test are read from
const TEST_ARGS_FILENAME: &str = "test.args";

/// Run one or all of the flowsamples by typing `flowsamples` or `cargo run -p flowsamples`
/// at the command line
/// - If no additional argument is provided, then all flowsamples found are executed
///   e.g. `flowsamples` or `cargo run -p flowsamples`
/// - If the name of a flow sample is provided as an additional argument, then that sample will be run
///   e.g. `flowsamples hello-world` or `cargo run -p flowsamples -- hello-world`
fn main() -> io::Result<()> {
    println!("`flowsample` version {}", env!("CARGO_PKG_VERSION"));
    println!(
        "Current Working Directory: `{}`",
        env::current_dir().expect("Could not get working directory").display()
    );

    let samples_root = env!("CARGO_MANIFEST_DIR");
    let samples_dir = Path::new(samples_root);
    let root_dir = samples_dir.parent().expect("Could not get parent directory");
    let samples_out_dir = root_dir.join("target/flowsamples");

    println!("Samples Root Directory: `{samples_root}`");

    let args: Vec<String> = env::args().collect();

    match args.len() {
        1 => {
            for entry in fs::read_dir(samples_root)? {
                let e = entry?;
                if e.file_type()?.is_dir() && e.path().join("root.toml").exists() {
                    run_sample(&e.path(), &samples_out_dir.join(e.file_name()), false)?
                }
            }
        }
        2 => {
            run_sample(&samples_dir.join(&args[1]), &samples_out_dir.join(&args[1]), false)?
        }
        _ => eprintln!("Usage: {} <optional_sample_directory_name>", args[0]),
    }

    Ok(())
}

/// Run one specific flow sample
fn run_sample(sample_dir: &Path, output_dir: &Path, flowrex: bool) -> io::Result<()> {
    let manifest_path = output_dir.join("manifest.json");
    println!("\n\tRunning Sample: {:?}", sample_dir.file_name());
    assert!(manifest_path.exists(), "Manifest not found at '{}'", manifest_path.display());
    println!("\tOutput written {}/", output_dir.display());
    println!("\t\tSTDIN is read from {TEST_STDIN_FILENAME}");
    println!("\t\tArguments are read from {TEST_ARGS_FILENAME}");
    println!("\t\tSTDOUT is sent to {TEST_STDOUT_FILENAME}");
    println!("\t\tSTDERR to {TEST_STDERR_FILENAME}");
    println!("\t\tFile output to {TEST_FILE_FILENAME}");

    let mut command_args: Vec<String> = vec!["--native".into()];

    if flowrex {
        command_args.push("--context".into())
    }

    command_args.push( manifest_path.display().to_string());

    command_args.append(&mut args(sample_dir)?);

    let output = File::create(output_dir.join(TEST_STDOUT_FILENAME))
        .expect("Could not get directory as string");
    let error = File::create(output_dir.join(TEST_STDERR_FILENAME))
        .expect("Could not get directory as string");

    let flowrex_child = if flowrex {
        match Command::new("flowrex").spawn() {
            Ok(child) => Some(child),
            Err(e) => return match e.kind() {
                ErrorKind::NotFound => Err(io::Error::new(
                    ErrorKind::Other,
                    format!("`flowrex` was not found! Check your $PATH. {e}"),
                )),
                _ => Err(io::Error::new(
                    ErrorKind::Other,
                    format!("Unexpected error running `flowrex`: {e}"),
                )),
            },
        }
    } else {
        None
    };

    println!("\tCommand line: 'flowr {}'", command_args.join(" "));
    match Command::new("flowr")
        .args(command_args)
        .current_dir(output_dir.canonicalize()?)
        .stdin(Stdio::piped())
        .stdout(Stdio::from(output))
        .stderr(Stdio::from(error))
        .spawn()
    {
        Ok(mut flowr_child) => {
            let stdin_file = sample_dir.join(TEST_STDIN_FILENAME);
            if stdin_file.exists() {
                let _ = Command::new("cat")
                    .args(vec![stdin_file])
                    .stdout(flowr_child.stdin.take().ok_or_else(|| {
                        io::Error::new(
                            ErrorKind::Other,
                            "Could not take STDIN of `flowr` process",
                        )
                    })?)
                    .spawn();
            }

            flowr_child.wait_with_output()?;
        }
        Err(e) => return match e.kind() {
            ErrorKind::NotFound => Err(io::Error::new(
                ErrorKind::Other,
                format!("`flowr` was not found! Check your $PATH. {e}"),
            )),
            _ => Err(io::Error::new(
                ErrorKind::Other,
                format!("Unexpected error running `flowr`: {e}"),
            )),
        },
    }

    // If flowrex was started - then kill it
    if let Some(mut child) = flowrex_child {
        println!("Killing 'flowrex'");
        child.kill().expect("Failed to kill server child process");
    }

    Ok(())
}

/// Read the flow args from a file and return them as a Vector of Strings that will be passed to `flowr`
fn args(sample_dir: &Path) -> io::Result<Vec<String>> {
    let args_file = sample_dir.join(TEST_ARGS_FILENAME);

    let mut args = Vec::new();

    // read args from the file if it exists, otherwise no args
    if let Ok(f) = File::open(args_file) {
        let f = BufReader::new(f);

        for line in f.lines() {
            args.push(line?);
        }
    }

    Ok(args)
}

#[cfg(test)]
mod test {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::{Command, Stdio};

    use serial_test::serial;

    use crate::{EXPECTED_FILE_FILENAME, EXPECTED_STDOUT_FILENAME, STDERR_FILENAME, TEST_FILE_FILENAME, TEST_STDOUT_FILENAME};

    fn test_sample(name: &str, flowrex: bool) {
        let samples_root = env!("CARGO_MANIFEST_DIR");
        let samples_dir = Path::new(samples_root);
        let sample_dir = samples_dir.join(name);

        let root_dir = samples_dir.parent().expect("Could not get parent directory");
        let samples_out_dir = root_dir.join("target/flowsamples");
        let output_dir = samples_out_dir.join(name);

        // Remove any previous output
        let _ = fs::remove_file(output_dir.join(STDERR_FILENAME));
        let _ = fs::remove_file(output_dir.join(TEST_FILE_FILENAME));
        let _ = fs::remove_file(output_dir.join(TEST_STDOUT_FILENAME));

        super::run_sample(&sample_dir, &output_dir, flowrex)
            .expect("Running of test sample failed");

        check_test_output(&sample_dir, &output_dir);

        // if test passed, remove output
        let _ = fs::remove_file(output_dir.join(STDERR_FILENAME));
        let _ = fs::remove_file(output_dir.join(TEST_FILE_FILENAME));
        let _ = fs::remove_file(output_dir.join(TEST_STDOUT_FILENAME));
    }

    fn compare_and_fail(expected_path: PathBuf, actual_path: PathBuf) {
        if expected_path.exists() {
            let diff = Command::new("diff")
                .args(vec![&expected_path, &actual_path])
                .stdin(Stdio::inherit())
                .stderr(Stdio::inherit())
                .stdout(Stdio::inherit())
                .spawn()
                .expect("Could not get child process");
            let output = diff.wait_with_output().expect("Could not get child process output");
            if output.status.success() {
                return;
            }
            panic!("Contents of '{}' doesn't match the expected contents in '{}'",
                           actual_path.display(), expected_path.display());
        }
    }

    fn check_test_output(sample_dir: &Path, output_dir: &Path) {
        let error_output = output_dir.join(STDERR_FILENAME);
        if error_output.exists() {
            let contents = fs::read_to_string(&error_output).expect("Could not read from {STDERR_FILENAME} file");

            if !contents.is_empty() {
                panic!(
                    "Sample {:?} produced output to STDERR written to '{}'\n{contents}",
                    sample_dir.file_name().expect("Could not get directory file name"),
                    error_output.display());
            }
        }

        compare_and_fail(sample_dir.join(EXPECTED_STDOUT_FILENAME), output_dir.join(TEST_STDOUT_FILENAME));
        compare_and_fail(sample_dir.join(EXPECTED_FILE_FILENAME), output_dir.join(TEST_FILE_FILENAME));
    }

    #[test]
    #[serial]
    fn test_args() {
        test_sample("args", false);
    }

    #[test]
    #[serial]
    fn test_arrays() {
        test_sample("arrays", false);
    }

    #[test]
    #[serial]
    fn test_factorial() {
        test_sample("factorial", false);
    }

    #[test]
    #[serial]
    fn test_fibonacci() {
        test_sample("fibonacci", false);
    }

    #[test]
    #[serial]
    fn test_fibonacci_flowrex() {
        test_sample("fibonacci", true);
    }

    #[test]
    #[serial]
    fn test_hello_world() {
        test_sample("hello-world", false);
    }

    #[test]
    #[serial]
    fn test_matrix_mult() {
        test_sample("matrix_mult", false);
    }

    #[test]
    #[serial]
    fn test_pipeline() {
        test_sample("pipeline", false);
    }

    #[test]
    #[serial]
    fn test_primitives() {
        test_sample("primitives", false);
    }

    #[test]
    #[serial]
    fn test_sequence() {
        test_sample("sequence", false);
    }

    #[test]
    #[serial]
    fn test_sequence_of_sequences() {
        test_sample("sequence-of-sequences", false);
    }

    #[test]
    #[serial]
    fn test_router() {
        test_sample("router", false);
    }

    #[test]
    #[serial]
    fn test_tokenizer() {
        test_sample("tokenizer", false);
    }

    // This sample uses provided implementations and hence is executing WASM
    #[test]
    #[serial]
    fn test_reverse_echo() {
        test_sample("reverse-echo", false);
    }

    // This sample uses provided implementations and hence is executing WASM
    #[test]
    #[serial]
    fn test_mandlebrot() {
        test_sample("mandlebrot", false);
    }

    #[test]
    #[serial]
    fn test_prime() {
        test_sample("prime", false);
    }
}

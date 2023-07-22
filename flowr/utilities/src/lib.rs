use std::{env, fs, io};
use std::fs::File;
use std::io::{BufRead, BufReader, ErrorKind};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Name of file where any Stdout will be written while executing a flowsample in test mode
const TEST_STDOUT_FILENAME: &str = "test.stdout";

/// Name of file where the Stdout is defined
const EXPECTED_STDOUT_FILENAME : &str = "expected.stdout";

/// Name of file where any Stdin will be read from while executing a flowsample in test mode
const TEST_STDIN_FILENAME : &str = "test.stdin";

/// Name of file where any Stderr will be written from while executing a flowsample in test mode
const TEST_STDERR_FILENAME : &str = "test.stderr";

/// Name of file used for file output of a sample
const TEST_FILE_FILENAME: &str = "test.file";

/// Name of file where expected file output is defined
const EXPECTED_FILE_FILENAME : &str = "expected.file";

/// Name of file where flow arguments for a flow sample test are read from
const TEST_ARGS_FILENAME: &str = "test.args";

// Compile a flow sample in-place in the `sample_dir` directory using flowc
fn compile_sample(sample_path: &Path) {
    let sample_dir = sample_path.to_string_lossy();
    let mut command = Command::new("flowc");
    // -d for debug symbols
    // -g to dump graphs
    // -c to skip running and only compile the flow
    // -O to optimize the WASM files generated
    // -C <dir> to set the context root dir
    // <sample_dir> is the path to the directory of the sample flow to compile
    let context_root = get_context_root().expect("Could not get context root");
    let command_args = vec!["-d", "-g", "-c", "-O",
                            "-C", &context_root,
                            &sample_dir];

    match command.args(&command_args).status() {
        Ok(stat) => {
            if !stat.success() {
                eprintln!("Error building sample, command line\n flowc {}",
                          command_args.join(" "));
                std::process::exit(1);
            }
        }
        Err(err) => {
            eprintln!("'{}' running command 'flowc {}'", err, command_args.join(" "));
            std::process::exit(1);
        }
    }
}

fn get_context_root() -> Result<String, String> {
    let context_root = match env::var("FLOW_CONTEXT_ROOT") {
        Ok(var) => PathBuf::from(&var),
        Err(_) => {
            let samples_dir = Path::new(env!("CARGO_MANIFEST_DIR")).parent()
                .ok_or("Could not get parent dir")?;
            samples_dir.join("src/bin/flowrcli/context")
        }
    };
    assert!(context_root.exists(), "Context root directory '{}' does not exist", context_root.display());
    Ok(context_root.to_str().expect("Could not convert path to String").to_string())
}

/// Run one specific flow sample
pub fn run_sample(sample_dir: &Path, flowrex: bool, native: bool) -> io::Result<()> {
    compile_sample(sample_dir);

    let manifest_path = sample_dir.join("manifest.json");
    println!("\n\tRunning Sample: {:?}", sample_dir.file_name());
    assert!(manifest_path.exists(), "Manifest not found at '{}'", manifest_path.display());
    println!("\t\tSTDIN is read from {TEST_STDIN_FILENAME}");
    println!("\t\tArguments are read from {TEST_ARGS_FILENAME}");
    println!("\t\tSTDOUT is sent to {TEST_STDOUT_FILENAME}");
    println!("\t\tSTDERR to {TEST_STDERR_FILENAME}");
    println!("\t\tFile output to {TEST_FILE_FILENAME}");

    let mut command_args: Vec<String> = if native {
        vec!["--native".into()]
    } else {
        vec![]
    };

    if flowrex {
        // set 0 executor threads in flowr coordinator, so that all job execution is done in flowrex
        command_args.push("--threads".into());
        command_args.push("0".into());
    }

    command_args.push( manifest_path.display().to_string());

    command_args.append(&mut args(sample_dir)?);

    // Remove any previous output
    let _ = fs::remove_file(sample_dir.join(TEST_STDERR_FILENAME));
    let _ = fs::remove_file(sample_dir.join(TEST_FILE_FILENAME));
    let _ = fs::remove_file(sample_dir.join(TEST_STDOUT_FILENAME));

    let output = File::create(sample_dir.join(TEST_STDOUT_FILENAME))
        .expect("Could not create Test StdOutput File");
    let error = File::create(sample_dir.join(TEST_STDERR_FILENAME))
        .expect("Could not create Test StdError File ");

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
    match Command::new("flowrcli")
        .args(command_args)
        .current_dir(sample_dir.canonicalize()?)
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

/// Run an example and check the output matches the expected
pub fn test_example(sample_dir: &Path, flowrex: bool, native: bool) {
    run_sample(sample_dir, flowrex, native).expect("Running of example failed");
    check_test_output(sample_dir);
}

fn check_test_output(sample_dir: &Path) {
    let error_output = sample_dir.join(TEST_STDERR_FILENAME);
    if error_output.exists() {
        let contents = fs::read_to_string(&error_output).expect("Could not read from {STDERR_FILENAME} file");

        if !contents.is_empty() {
            panic!(
                "Sample {:?} produced output to STDERR written to '{}'\n{contents}",
                sample_dir.file_name().expect("Could not get directory file name"),
                error_output.display());
        }
    }

    compare_and_fail(sample_dir.join(EXPECTED_STDOUT_FILENAME), sample_dir.join(TEST_STDOUT_FILENAME));
    compare_and_fail(sample_dir.join(EXPECTED_FILE_FILENAME), sample_dir.join(TEST_FILE_FILENAME));
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

#[test]
#[serial]
fn test_args() {
    test_example("args", false, true);
}

#[test]
#[serial]
fn test_arrays() {
    test_example("arrays", false, true);
}

#[test]
#[serial]
fn test_factorial() {
    test_example("factorial", false, true);
}

#[test]
#[serial]
fn test_fibonacci() {
    test_example("fibonacci", false, true);
}

#[test]
#[serial]
fn test_fibonacci_wasm() {
    test_example("fibonacci", false, false);
}

#[test]
#[serial]
fn test_fibonacci_flowrex() {
    test_example("fibonacci", true, true);
}

#[test]
#[serial]
fn test_hello_world() {
}

#[test]
#[serial]
fn test_matrix_mult() {
    test_example("matrix_mult", false, true);
}

#[test]
#[serial]
fn test_pipeline() {
    test_example("pipeline", false, true);
}

#[test]
#[serial]
fn test_primitives() {
    test_example("primitives", false, true);
}

#[test]
#[serial]
fn test_sequence() {
    test_example("sequence", false, true);
}

#[test]
#[serial]
fn test_sequence_of_sequences() {
    test_example("sequence-of-sequences", false, true);
}

#[test]
#[serial]
fn test_router() {
    test_example("router", false, true);
}

#[test]
#[serial]
fn test_tokenizer() {
    test_example("tokenizer", false, true);
}

// This sample uses provided implementations and hence is executing WASM
#[test]
#[serial]
fn test_reverse_echo() {
    test_example("reverse-echo", false, true);
}

// This sample uses provided implementations and hence is executing WASM
#[test]
#[serial]
fn test_mandlebrot() {
    test_example("mandlebrot", false, true);
}

#[test]
#[serial]
fn test_prime() {
    test_example("prime", false, true);
}

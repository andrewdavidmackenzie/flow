use std::{env, fs, io};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Name of file where any Stdout will be written while executing an example
const TEST_STDOUT_FILENAME: &str = "test.stdout";

/// Name of file where the Stdout is defined
const EXPECTED_STDOUT_FILENAME : &str = "expected.stdout";

/// Name of file where any Stdin will be read from while executing am example
const TEST_STDIN_FILENAME : &str = "test.stdin";

/// Name of file where any Stderr will be written from while executing an example
const TEST_STDERR_FILENAME : &str = "test.stderr";

/// Name of file used for file output of a example
const TEST_FILE_FILENAME: &str = "test.file";

/// Name of file where expected file output is defined
const EXPECTED_FILE_FILENAME : &str = "expected.file";

/// Name of file where flow arguments for a flow example test are read from
const TEST_ARGS_FILENAME: &str = "test.args";

/// Run one specific flow example
pub fn run_example(source_file: &str, runner: &str, flowrex: bool, native: bool) {
    let mut sample_dir = PathBuf::from(source_file);
    sample_dir.pop();

    compile_sample(&sample_dir, runner);

    println!("\n\tRunning example: {}", sample_dir.display());
    println!("\t\tRunner: {}", runner);
    println!("\t\tSTDIN is read from {TEST_STDIN_FILENAME}");
    println!("\t\tArguments are read from {TEST_ARGS_FILENAME}");
    println!("\t\tSTDOUT is sent to {TEST_STDOUT_FILENAME}");
    println!("\t\tSTDERR to {TEST_STDERR_FILENAME}");
    println!("\t\tFile output to {TEST_FILE_FILENAME}");

    // Remove any previous output
    let _ = fs::remove_file(sample_dir.join(TEST_STDERR_FILENAME));
    let _ = fs::remove_file(sample_dir.join(TEST_FILE_FILENAME));
    let _ = fs::remove_file(sample_dir.join(TEST_STDOUT_FILENAME));

    let mut command_args: Vec<String> = if native {
        vec!["--native".into()]
    } else {
        vec![]
    };

    let flowrex_child = if flowrex {
        // set 0 executor threads in flowr coordinator, so that all job execution is done in flowrex
        command_args.push("--threads".into());
        command_args.push("0".into());
        Some(Command::new("flowrex").spawn().expect("Could not spawn flowrex"))
    } else {
        None
    };

    command_args.push( "manifest.json".into());
    command_args.append(&mut args(&sample_dir).expect("Could not get flow args"));

    let output = File::create(sample_dir.join(TEST_STDOUT_FILENAME))
        .expect("Could not create Test StdOutput File");
    let error = File::create(sample_dir.join(TEST_STDERR_FILENAME))
        .expect("Could not create Test StdError File ");

    println!("\tCommand line: '{} {}'",runner, command_args.join(" "));
    let mut runner_child = Command::new(runner)
        .args(command_args)
        .current_dir(sample_dir.canonicalize().expect("Could not canonicalize path"))
        .stdin(Stdio::piped())
        .stdout(Stdio::from(output))
        .stderr(Stdio::from(error))
        .spawn().expect("Could not spawn runner");

    let stdin_file = sample_dir.join(TEST_STDIN_FILENAME);
    if stdin_file.exists() {
        let _ = Command::new("cat")
            .args(vec![stdin_file])
            .stdout(runner_child.stdin.take().expect("Could not take STDIN"))
            .spawn();
    }

    runner_child.wait_with_output().expect("Could not get sub process output");

    // If flowrex was started - then kill it
    if let Some(mut child) = flowrex_child {
        println!("Killing 'flowrex'");
        child.kill().expect("Failed to kill server child process");
    }
}

/// Run an example and check the output matches the expected
///
/// It turns out that when an example is run (`cargo run --example name`), the CWD is the workspace
/// root (./flow)
/// When an example's test (single test) is being run (`cargo test --example name`), the CWD is the
/// workspace member crate root (./flow/flowr)
/// However, if there are multiple tests in the example, then the CWD is the workspace root (./flow)
///
/// So, we have a flag to indicate if the process should change the CWD to the parent directory or
/// not, before running the example
pub fn test_example(source_file: &str, runner: &str, flowrex: bool, native: bool, parent_dir: bool) {
    if parent_dir {
        let _ = env::set_current_dir(env::current_dir()
            .expect("Could not get current directory")
            .parent()
            .expect("Could not CD to parent directory"));

        run_example(source_file, runner, flowrex, native);
        check_test_output(source_file);
    }
}

/// Read the flow args from a file and return them as a Vector of Strings that will be passed in
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

// Compile a flow example in-place in the `sample_dir` directory using flowc
fn compile_sample(sample_path: &Path, runner: &str) {
    let sample_dir = sample_path.to_string_lossy();
    let context_root = get_context_root(runner).expect("Could not get context root");

    let mut command = Command::new("flowc");
    // -d for debug symbols
    // -g to dump graphs
    // -c to skip running and only compile the flow
    // -O to optimize the WASM files generated
    // -C <dir> to set the context root dir
    // <sample_dir> is the path to the directory of the sample flow to compile
    let command_args = vec!["-d", "-g", "-c", "-O",
                            "-C", &context_root,
                            &sample_dir];

    let stat = command
        .args(&command_args)
        .status().expect("Could not get status of 'flowc' execution");

    if !stat.success() {
        panic!("Error building example, command line\n flowc {}", command_args.join(" "));
    }
}

fn get_context_root(runner: &str) -> Result<String, String> {
    let context_root = match env::var("FLOW_CONTEXT_ROOT") {
        Ok(var) => PathBuf::from(&var),
        Err(_) => {
            let samples_dir = Path::new(env!("CARGO_MANIFEST_DIR")).parent()
                .ok_or("Could not get parent dir")?;
            samples_dir.join(format!("src/bin/{}/context", runner))
        }
    };
    assert!(context_root.exists(), "Context root directory '{}' does not exist", context_root.display());
    Ok(context_root.to_str().expect("Could not convert path to String").to_string())
}

fn check_test_output(source_file: &str) {
    let mut sample_dir = PathBuf::from(source_file);
    sample_dir.pop();

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
use std::{env, fs, io};
use std::fmt::Write;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::io::Read;
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

    compile_example(&sample_dir, runner);

    println!("\n\tRunning example: {}", sample_dir.display());
    println!("\t\tRunner: {}", runner);
    println!("\t\tSTDIN is read from {TEST_STDIN_FILENAME}");
    println!("\t\tArguments are read from {TEST_ARGS_FILENAME}");
    println!("\t\tSTDOUT is saved in {TEST_STDOUT_FILENAME}");
    println!("\t\tSTDERR is saved in {TEST_STDERR_FILENAME}");
    println!("\t\tFile output is saved in {TEST_FILE_FILENAME}");

    // Remove any previous output
    let _ = fs::remove_file(sample_dir.join(TEST_STDERR_FILENAME));
    let _ = fs::remove_file(sample_dir.join(TEST_FILE_FILENAME));
    let _ = fs::remove_file(sample_dir.join(TEST_STDOUT_FILENAME));

    let mut runner_args: Vec<String> = if native {
        vec!["--native".into()]
    } else {
        vec![]
    };

    if runner == "flowrgui" {
        runner_args.push("--auto".into());
    }

    let flowrex_child = if flowrex {
        // set 0 executor threads in flowr coordinator, so that all job execution is done in flowrex
        runner_args.push("--threads".into());
        runner_args.push("0".into());
        Some(Command::new("flowrex").spawn().expect("Could not spawn flowrex"))
    } else {
        None
    };

    runner_args.push( "manifest.json".into());
    runner_args.append(&mut args(&sample_dir).expect("Could not get flow args"));

    let output = File::create(sample_dir.join(TEST_STDOUT_FILENAME))
        .expect("Could not create Test StdOutput File");
    let error = File::create(sample_dir.join(TEST_STDERR_FILENAME))
        .expect("Could not create Test StdError File ");

    println!("\tCommand line: '{} {}'", runner, runner_args.join(" "));
    let mut runner_child = Command::new(runner)
        .args(runner_args)
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
pub fn test_example(source_file: &str, runner: &str, flowrex: bool, native: bool) {
    let _ = env::set_current_dir(PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().expect("Could not cd into flowr directory")
        .parent().expect("Could not cd into flow directory"));

    run_example(source_file, runner, flowrex, native);
    check_test_output(source_file);
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

/// Compile a flow example in-place in the `sample_dir` directory using flowc
pub fn compile_example(sample_path: &Path, runner: &str) {
    let sample_dir = sample_path.to_string_lossy();

    let mut command = Command::new("flowc");
    // -d for debug symbols
    // -g to dump graphs
    // -c to skip running and only compile the flow
    // -O to optimize the WASM files generated
    // -r <runner> to specify the runner to use
    // <sample_dir> is the path to the directory of the sample flow to compile
    let command_args = vec!["-d", "-g", "-c", "-O", "-r", runner, &sample_dir];

    let stat = command
        .args(&command_args)
        .status().expect("Could not get status of 'flowc' execution");

    if !stat.success() {
        panic!("Error building example, command line\n flowc {}", command_args.join(" "));
    }
}

pub fn check_test_output(source_file: &str) {
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
        let output = diff.wait_with_output().expect("Could not get diff output");
        if output.status.success() {
            return;
        }
        panic!("Contents of '{}' doesn't match the expected contents in '{}'",
               actual_path.display(), expected_path.display());
    }
}

/// Execute a flow using separate server (coordinator) and client
pub fn execute_flow_client_server(example_name: &str, manifest: PathBuf) {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root_dir = crate_dir.parent().expect("Could not go to parent flowr dir");
    let samples_dir = root_dir.join("examples").join(example_name);

    let mut server_command = Command::new("flowrcli");

    // separate 'flowr' server process args: -n for native libs, -s to get a server process
    let server_args = vec!["-n", "-s"];

    println!("Starting 'flowrcli' as server with command line: 'flowrcli {}'", server_args.join(" "));

    // spawn the 'flowr' server process
    let mut server = server_command
        .args(server_args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn flowrcli");

    // capture the discovery port by reading one line of stdout
    let stdout = server.stdout.as_mut().expect("Could not read stdout of server");
    let mut reader = BufReader::new(stdout);
    let mut discovery_port = String::new();
    reader.read_line(&mut discovery_port).expect("Could not read line");

    let mut client = Command::new("flowrcli");
    let manifest_str = manifest.to_string_lossy();
    let client_args =  vec!["-c", discovery_port.trim(), &manifest_str];
    println!("Starting 'flowrcli' client with command line: 'flowr {}'", client_args.join(" "));

    // spawn the 'flowrcli' client process
    let mut runner = client
        .args(client_args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Could not spawn flowrcli process");

    // read it's stderr - don't fail, to ensure we kill the server
    let mut actual_stderr = String::new();
    if let Some(ref mut stderr) = runner.stderr {
        for line in BufReader::new(stderr).lines() {
            let _ = writeln!(actual_stderr, "{}", &line.expect("Could not read line"));
        }
    }

    // read it's stdout - don't fail, to ensure we kill the server
    let mut actual_stdout = String::new();
    if let Some(ref mut stdout) = runner.stdout {
        for line in BufReader::new(stdout).lines() {
            let _ = writeln!(actual_stdout, "{}", &line.expect("Could not read line"));
        }
    }

    println!("Killing 'flowr' server");
    server.kill().expect("Failed to kill server child process");

    if !actual_stderr.is_empty() {
        eprintln!("STDERR: {actual_stderr}");
        panic!("Failed due to STDERR output")
    }

    let expected_stdout = read_file(&samples_dir, "expected.stdout");
    if expected_stdout != actual_stdout {
        println!("Expected STDOUT:\n{expected_stdout}");
        println!("Actual STDOUT:\n{actual_stdout}");
        panic!("Actual STDOUT did not match expected.stdout");
    }
}

fn read_file(test_dir: &Path, file_name: &str) -> String {
    let expected_file = test_dir.join(file_name);
    if !expected_file.exists() {
        return String::new();
    }

    let mut f = File::open(&expected_file).expect("Could not open file");
    let mut buffer = Vec::new();
    f.read_to_end(&mut buffer).expect("Could not read from file");
    String::from_utf8(buffer).expect("Could not convert to String")
}
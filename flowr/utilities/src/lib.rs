use std::fmt::Write;
use std::fs::File;
use std::io::Read;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::{env, fs, io};

/// Normalize text output for cross-platform comparison.
/// Converts Windows `\r\n` line endings to Unix `\n` and trims surrounding whitespace.
fn normalize_output(s: &str) -> String {
    s.replace("\r\n", "\n").trim().to_string()
}

/// Name of file where any Stdout will be written while executing an example
const TEST_STDOUT_FILENAME: &str = "test.stdout";

/// Name of file where the Stdout is defined (ordered comparison)
const EXPECTED_STDOUT_FILENAME: &str = "expected.stdout";

/// Name of file where the Stdout is defined (unordered line-set comparison)
const EXPECTED_UNORDERED_STDOUT_FILENAME: &str = "expected_unordered.stdout";

/// Name of file where any Stdin will be read from while executing am example
const TEST_STDIN_FILENAME: &str = "test.stdin";

/// Name of file where any Stderr will be written from while executing an example
const TEST_STDERR_FILENAME: &str = "test.stderr";

/// Name of file used for file output of a example
const TEST_FILE_FILENAME: &str = "test.file";

/// Name of file where expected file output is defined
const EXPECTED_FILE_FILENAME: &str = "expected.file";

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
        Some(
            Command::new("flowrex")
                .spawn()
                .expect("Could not spawn flowrex"),
        )
    } else {
        None
    };

    runner_args.push("manifest.json".into());
    runner_args.append(&mut args(&sample_dir).expect("Could not get flow args"));

    let output = File::create(sample_dir.join(TEST_STDOUT_FILENAME))
        .expect("Could not create Test StdOutput File");

    let error = File::create(sample_dir.join(TEST_STDERR_FILENAME))
        .expect("Could not create Test StdError File ");
    let stderr_target = Stdio::from(error);

    println!("\tCommand line: '{} {}'", runner, runner_args.join(" "));
    let mut runner_child = Command::new(runner)
        .args(runner_args)
        .current_dir(
            sample_dir
                .canonicalize()
                .expect("Could not canonicalize path"),
        )
        .stdin(Stdio::piped())
        .stdout(Stdio::from(output))
        .stderr(stderr_target)
        .spawn()
        .expect("Could not spawn runner");

    let stdin_file = sample_dir.join(TEST_STDIN_FILENAME);
    if stdin_file.exists() {
        if let Some(mut child_stdin) = runner_child.stdin.take() {
            let content = fs::read(&stdin_file).expect("Could not read stdin file");
            std::io::Write::write_all(&mut child_stdin, &content)
                .expect("Could not write to child stdin");
        }
    }

    runner_child
        .wait_with_output()
        .expect("Could not get sub process output");

    // If flowrex was started - then kill it
    if let Some(mut child) = flowrex_child {
        println!("Killing 'flowrex'");
        child.kill().expect("Failed to kill server child process");
        child.wait().expect("Failed to wait for child to exit");
    }
}

/// Run an example and check the output matches the expected
pub fn test_example(source_file: &str, runner: &str, flowrex: bool, native: bool) {
    let _ = env::set_current_dir(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("Could not cd into flowr directory")
            .parent()
            .expect("Could not cd into flow directory"),
    );

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
        .status()
        .expect("Could not get status of 'flowc' execution");

    if !stat.success() {
        panic!(
            "Error building example, command line\n flowc {}",
            command_args.join(" ")
        );
    }
}

pub fn check_test_output(source_file: &str) {
    let mut sample_dir = PathBuf::from(source_file);
    sample_dir.pop();

    let error_output = sample_dir.join(TEST_STDERR_FILENAME);
    if error_output.exists() {
        let contents =
            fs::read_to_string(&error_output).expect("Could not read from {STDERR_FILENAME} file");

        if !contents.is_empty() {
            panic!(
                "Sample {:?} produced output to STDERR written to '{}'\n{contents}",
                sample_dir
                    .file_name()
                    .expect("Could not get directory file name"),
                error_output.display()
            );
        }
    }

    let unordered_path = sample_dir.join(EXPECTED_UNORDERED_STDOUT_FILENAME);
    if unordered_path.exists() {
        compare_unordered_and_fail(unordered_path, sample_dir.join(TEST_STDOUT_FILENAME));
    } else {
        compare_and_fail(
            sample_dir.join(EXPECTED_STDOUT_FILENAME),
            sample_dir.join(TEST_STDOUT_FILENAME),
        );
    }
    compare_and_fail(
        sample_dir.join(EXPECTED_FILE_FILENAME),
        sample_dir.join(TEST_FILE_FILENAME),
    );
}

fn compare_and_fail(expected_path: PathBuf, actual_path: PathBuf) {
    if expected_path.exists() {
        let expected = fs::read(&expected_path).expect("Could not read expected file");
        let actual = fs::read(&actual_path).expect("Could not read actual file");
        if expected == actual {
            return;
        }
        // Try text comparison with normalized line endings
        if let (Ok(exp_str), Ok(act_str)) =
            (std::str::from_utf8(&expected), std::str::from_utf8(&actual))
        {
            let exp_normalized = normalize_output(exp_str);
            let act_normalized = normalize_output(act_str);
            if exp_normalized == act_normalized {
                return;
            }
            eprintln!("Expected:\n{exp_normalized}");
            eprintln!("Actual:\n{act_normalized}");
        }
        panic!(
            "Contents of '{}' doesn't match the expected contents in '{}'",
            actual_path.display(),
            expected_path.display()
        );
    }
}

fn compare_unordered_and_fail(expected_path: PathBuf, actual_path: PathBuf) {
    if expected_path.exists() {
        let expected_raw =
            fs::read_to_string(&expected_path).expect("Could not read expected file");
        let actual_raw = fs::read_to_string(&actual_path).expect("Could not read actual file");
        let expected = normalize_output(&expected_raw);
        let actual = normalize_output(&actual_raw);

        let mut expected_lines: Vec<&str> = expected.lines().filter(|l| !l.is_empty()).collect();
        let mut actual_lines: Vec<&str> = actual.lines().filter(|l| !l.is_empty()).collect();

        expected_lines.sort();
        actual_lines.sort();

        if expected_lines != actual_lines {
            let missing: Vec<&&str> = expected_lines
                .iter()
                .filter(|l| !actual_lines.contains(l))
                .collect();
            let extra: Vec<&&str> = actual_lines
                .iter()
                .filter(|l| !expected_lines.contains(l))
                .collect();

            let mut msg = format!(
                "Unordered comparison of '{}' vs '{}' failed.\n",
                actual_path.display(),
                expected_path.display()
            );
            if !missing.is_empty() {
                let _ = write!(msg, "  Missing lines: {missing:?}\n");
            }
            if !extra.is_empty() {
                let _ = write!(msg, "  Extra lines: {extra:?}\n");
            }
            panic!("{msg}");
        }
    }
}

/// Execute a flow using separate server (coordinator) and client
pub fn execute_flow_client_server(example_name: &str, manifest: PathBuf) {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root_dir = crate_dir
        .parent()
        .expect("Could not go to parent flowr dir");
    let samples_dir = root_dir.join("examples").join(example_name);

    let mut server_command = Command::new("flowrcli");

    // separate 'flowr' server process args: -n for native libs, -s to get a server process
    let server_args = vec!["-n", "-s"];

    println!(
        "Starting 'flowrcli' as server with command line: 'flowrcli {}'",
        server_args.join(" ")
    );

    // spawn the 'flowr' server process
    let mut server = server_command
        .args(server_args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn flowrcli");

    // wait for the server to signal it's ready
    let stdout = server
        .stdout
        .as_mut()
        .expect("Could not read stdout of server");
    let mut reader = BufReader::new(stdout);
    let mut ready_line = String::new();
    let bytes_read = reader
        .read_line(&mut ready_line)
        .expect("Could not read ready line from server");
    assert!(
        bytes_read > 0 && ready_line.trim() == "ready",
        "Server did not report ready. First stdout line: {ready_line:?}"
    );

    let mut client = Command::new("flowrcli");
    let manifest_str = manifest.to_string_lossy();
    let client_args = vec!["-c", &manifest_str];
    println!(
        "Starting 'flowrcli' client with command line: 'flowr {}'",
        client_args.join(" ")
    );

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
    server.wait().expect("Failed to wait for child to exit");

    if !actual_stderr.is_empty() {
        eprintln!("STDERR: {actual_stderr}");
        panic!("Failed due to STDERR output")
    }

    let unordered_path = samples_dir.join(EXPECTED_UNORDERED_STDOUT_FILENAME);
    if unordered_path.exists() {
        let expected_stdout =
            normalize_output(&read_file(&samples_dir, EXPECTED_UNORDERED_STDOUT_FILENAME));
        let actual_normalized = normalize_output(&actual_stdout);
        let mut expected_lines: Vec<&str> =
            expected_stdout.lines().filter(|l| !l.is_empty()).collect();
        let mut actual_lines: Vec<&str> = actual_normalized
            .lines()
            .filter(|l| !l.is_empty())
            .collect();
        expected_lines.sort();
        actual_lines.sort();
        if expected_lines != actual_lines {
            println!("Expected lines (sorted): {expected_lines:?}");
            println!("Actual lines (sorted): {actual_lines:?}");
            panic!("Actual STDOUT lines did not match expected_unordered.stdout");
        }
    } else {
        let expected_stdout = normalize_output(&read_file(&samples_dir, "expected.stdout"));
        let actual_normalized = normalize_output(&actual_stdout);
        if expected_stdout != actual_normalized {
            println!("Expected STDOUT:\n{expected_stdout}");
            println!("Actual STDOUT:\n{actual_normalized}");
            panic!("Actual STDOUT did not match expected.stdout");
        }
    }
}

/// A debug session that manages flowrcli and flowrdb processes for integration testing.
///
/// Start a debug session with `DebugSession::start()`, send commands with `send()`,
/// and get flowrdb's output with `finish()`.
pub struct DebugSession {
    server: std::process::Child,
    flowrdb: std::process::Child,
    stdin: Option<std::process::ChildStdin>,
    stdout_lines: Vec<String>,
    stdout_reader: Option<BufReader<std::process::ChildStdout>>,
    server_stderr_rx: Option<std::sync::mpsc::Receiver<String>>,
}

impl DebugSession {
    /// Start a debug session on the given example directory.
    /// Spawns flowrcli with `--debugger --native` and connects flowrdb to it.
    /// Extra args (e.g. flow arguments) can be passed via `extra_args`.
    pub fn start(example_dir: &Path, extra_args: &[&str]) -> Self {
        Self::start_with_runner(example_dir, "flowrcli", extra_args)
    }

    /// Start a debug session using a specific runner (`flowrcli` or `flowrgui`).
    pub fn start_with_runner(example_dir: &Path, runner: &str, extra_args: &[&str]) -> Self {
        compile_example(example_dir, runner);

        let mut args = vec!["--debugger", "--native"];
        if runner == "flowrgui" {
            args.push("--auto");
            args.push("-v");
            args.push("info");
        }
        args.push("manifest.json");
        for arg in extra_args {
            args.push(arg);
        }

        let mut server = Command::new(runner)
            .args(&args)
            .current_dir(
                example_dir
                    .canonicalize()
                    .expect("Could not canonicalize example dir"),
            )
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap_or_else(|e| panic!("Could not spawn {runner}: {e}"));

        let stderr = server.stderr.take().expect("Could not get stderr");

        // Read stderr in a background thread — find the port line, then keep
        // collecting remaining stderr for diagnostics on failure
        let runner_name = runner.to_string();
        let (port_tx, port_rx) = std::sync::mpsc::channel();
        let (stderr_tx, stderr_rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut reader = BufReader::new(stderr);
            loop {
                let mut line = String::new();
                match reader.read_line(&mut line) {
                    Ok(0) => break,
                    Ok(_) => {
                        if line.contains("localhost:") {
                            let _ = port_tx.send(Ok(line.clone()));
                        }
                        let _ = stderr_tx.send(line);
                    }
                    Err(_) => break,
                }
            }
        });

        let port_line = port_rx
            .recv_timeout(std::time::Duration::from_secs(30))
            .unwrap_or_else(|_| {
                let _ = server.kill();
                Err(format!(
                    "Timed out waiting for {runner_name} to print debug port (30s)"
                ))
            })
            .unwrap_or_else(|e| panic!("{e}"));

        let port = port_line
            .split("localhost:")
            .nth(1)
            .and_then(|s| s.trim().parse::<u16>().ok())
            .unwrap_or_else(|| panic!("Could not parse debug port from: {port_line}"));

        let flowrdb = Command::new("flowrdb")
            .args(["--address", &format!("localhost:{port}")])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Could not spawn flowrdb");

        let mut session = DebugSession {
            server,
            flowrdb,
            stdin: None,
            stdout_lines: Vec::new(),
            stdout_reader: None,
            server_stderr_rx: Some(stderr_rx),
        };
        session.stdin = session.flowrdb.stdin.take();

        // Wait for the ZMQ handshake to complete by reading flowrdb stdout
        // until we see "Entering Debugger" which confirms the connection
        let stdout = session
            .flowrdb
            .stdout
            .take()
            .expect("Could not get flowrdb stdout");
        let mut stdout_reader = BufReader::new(stdout);
        let mut connected = false;
        loop {
            let mut line = String::new();
            stdout_reader
                .read_line(&mut line)
                .expect("Could not read from flowrdb stdout");
            if line.is_empty() {
                break;
            }
            let ready = line.contains("Entering Debugger");
            session.stdout_lines.push(line);
            if ready {
                connected = true;
                break;
            }
        }
        if !connected {
            // Give server a moment to flush stderr, then drain the channel
            std::thread::sleep(std::time::Duration::from_secs(1));
            let mut server_stderr_lines = Vec::new();
            if let Some(ref rx) = session.server_stderr_rx {
                while let Ok(line) = rx.try_recv() {
                    server_stderr_lines.push(line);
                }
            }
            let mut flowrdb_stderr = String::new();
            if let Some(ref mut stderr) = session.flowrdb.stderr {
                let _ = stderr.read_to_string(&mut flowrdb_stderr);
            }
            panic!(
                "flowrdb exited before completing debug handshake.\n\
                 flowrdb output:\n{}\n\
                 Server stderr:\n{}\n\
                 flowrdb stderr:\n{flowrdb_stderr}",
                session.stdout_lines.join(""),
                server_stderr_lines.join(""),
            );
        }
        session.stdout_reader = Some(stdout_reader);

        session
    }

    /// Send a debugger command (e.g. "s", "c", "h", "e")
    pub fn send(&mut self, command: &str) {
        use std::io::Write as _;
        if let Some(ref mut stdin) = self.stdin {
            writeln!(stdin, "{command}").expect("Could not write to flowrdb stdin");
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
    }

    /// Close stdin, wait for flowrdb to exit, and return its stdout
    pub fn finish(mut self) -> String {
        drop(self.stdin.take());

        if let Some(reader) = self.stdout_reader.take() {
            for line in reader.lines() {
                match line {
                    Ok(l) => self.stdout_lines.push(format!("{l}\n")),
                    Err(_) => break,
                }
            }
        }

        self.flowrdb.wait().expect("Could not wait for flowrdb");
        self.server.kill().expect("Could not kill flowrcli");
        self.server.wait().expect("Could not wait for flowrcli");
        self.stdout_lines.join("")
    }
}

fn read_file(test_dir: &Path, file_name: &str) -> String {
    let expected_file = test_dir.join(file_name);
    if !expected_file.exists() {
        return String::new();
    }

    let mut f = File::open(&expected_file).expect("Could not open file");
    let mut buffer = Vec::new();
    f.read_to_end(&mut buffer)
        .expect("Could not read from file");
    String::from_utf8(buffer).expect("Could not convert to String")
}

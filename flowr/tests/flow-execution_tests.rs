#![allow(clippy::result_large_err)]

#[macro_use]
extern crate error_chain;

use std::fmt::Write as FormatWrite;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::process::Stdio;

use serial_test::serial;
use tempdir::TempDir;

#[doc(hidden)]
mod errors {
    // Create the Error, ErrorKind, ResultExt, and Result types
    error_chain! {}
}

error_chain! {
    foreign_links {
        Io(::std::io::Error);
    }
}

/// Execution tests
///
/// These are a set of System tests that compile a sample flow, and then execute it, capturing
/// the output and comparing it to the expected output.
///
/// They depend on flowc being built already and in the users $PATH, in order that Command()
/// can find and execute it.
///
/// The same is true of flowr, but that is a binary from this crate, so cargo should build it
/// first, but it must be in users $PATH for flow execution.

fn test_args(test_dir: &Path) -> Option<Vec<String>> {
    let args_file = test_dir.join("test.args");
    let mut args = Vec::new();

    let file = File::open(args_file).ok()?;
    let f = BufReader::new(file);

    for line in f.lines() {
        args.push(line.expect("Could not read from file"));
    }

    Some(args)
}

fn read_file(test_dir: &Path, file_name: &str) -> String {
    let expected_file = test_dir.join(file_name);
    if !expected_file.exists() {
        return "".into();
    }

    let mut f = File::open(&expected_file).expect("Could not open file");
    let mut buffer = Vec::new();
    f.read_to_end(&mut buffer).expect("Could not read from file");
    String::from_utf8(buffer).expect("Could not convert to String")
}

fn execute_flow_client_server(test_name: &str, manifest: PathBuf) -> Result<()> {
    let root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let test_dir = root_dir.join("tests/test-flows").join(test_name);

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
    let stdout = server.stdout.as_mut().ok_or("Could not read stdout of server")?;
    let mut reader = BufReader::new(stdout);
    let mut discovery_port = String::new();
    reader.read_line(&mut discovery_port)?;

    let mut client = Command::new("flowrcli");
    let manifest_str = manifest.to_string_lossy();
    let client_args =  vec!["-c", discovery_port.trim(), &manifest_str];
    println!("Starting 'flowrcli' client with command line: 'flowr {}'", client_args.join(" "));

    // spawn the 'flowr' client process
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
    server.kill().map_err(|_| "Failed to kill server child process")?;

    if !actual_stderr.is_empty() {
        bail!(format!("STDERR: {actual_stderr}"))
    }

    let expected_stdout = read_file(&test_dir, "expected.stdout");
    if expected_stdout != actual_stdout {
        println!("Expected STDOUT:\n{expected_stdout}");
        println!("Actual STDOUT:\n{actual_stdout}");
        bail!("Actual STDOUT did not match expected.stdout");
    }

    Ok(())
}

fn compile_and_execute(runner_name: &str, test_name: &str, execute: bool) -> Result<PathBuf> {
    let root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let context_dir = root_dir.join(format!("src/bin/{}/context", runner_name));
    let context_dir_str = context_dir.to_string_lossy().to_string();
    let test_dir = root_dir.join("tests/test-flows").join(test_name);
    let test_dir_str = test_dir.to_string_lossy().to_string();
    let out_dir = TempDir::new("flow-execution-tests")
        .expect("A temp dir").into_path();
    let out_dir_str = out_dir.to_string_lossy().to_string();

    let mut command = Command::new("flowc");
    let mut command_args: Vec<String> = vec![
        "--context_root".into(), context_dir_str, // Use flow context root
        "--graphs".into(), "--optimize".into(), // Optimize flows
        "--output".into(), out_dir_str // Generate into {out_dir_str}
    ];

    if !execute {
        command_args.push("--compile".into());
    }

    let test_stdin_path = test_dir.join("test.stdin");
    let test_stdin_filename = test_stdin_path.to_string_lossy().to_string();
    if test_stdin_path.exists() {
        command_args.push("--stdin".into());
        command_args.push(test_stdin_filename);
    }

    command_args.push(test_dir_str);   // Compile and run this flow

    // Add any args to pass onto the flow
    if let Some(mut args) = test_args(&test_dir) {
        command_args.append(&mut args);
    }

    println!("Command line: 'flowc {}'", command_args.join(" "));

    let execution = command
        .args(command_args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| "Could not spawn flowc process")?;

    if execute {
        // read it's stderr
        let mut actual_stderr = String::new();
        let stderr = execution.stderr.ok_or("Could not get stderr")?;
        for line in BufReader::new(stderr).lines() {
            let _ = writeln!(actual_stderr, "{}", &line.expect("Could not read line"));
        }

        if !actual_stderr.is_empty() {
            bail!(format!("STDERR: {actual_stderr}"))
        }

        // read it's stdout
        let mut actual_stdout = String::new();
        let stdout = execution.stdout.ok_or("Could not get stdout")?;
        for line in BufReader::new(stdout).lines() {
            let _ = writeln!(actual_stdout, "{}", &line.expect("Could not read line"));
        }

        let expected_stdout = read_file(&test_dir, "expected.stdout");
        if expected_stdout != actual_stdout {
            bail!(format!("Expected STDOUT: {expected_stdout}\nActual STDOUT: {actual_stdout}"));
        }
    }

    Ok(out_dir.join("manifest.json"))
}

#[cfg(feature = "debugger")]
#[test]
#[serial]
fn debug_print_args() {
    compile_and_execute("flowrcli", "debug-print-args", true).expect("Test failed");
}

#[cfg(feature = "debugger")]
#[test]
#[serial]
fn debug_help_string() {
    compile_and_execute("flowrcli", "debug-help-string", true).expect("Test failed");
}

#[test]
#[serial]
fn print_args() {
    compile_and_execute("flowrcli", "print-args", true).expect("Test failed");
}

#[test]
#[serial]
fn hello_world() {
    compile_and_execute("flowrcli", "hello-world", true).expect("Test failed");
}

#[test]
#[serial]
fn line_echo() {
    compile_and_execute("flowrcli", "line-echo", true).expect("Test failed");
}

#[test]
#[serial]
fn args() {
    compile_and_execute("flowrcli", "args", true).expect("Test failed");
}

#[test]
#[serial]
fn args_json() {
    compile_and_execute("flowrcli", "args-json", true).expect("Test failed");
}

#[test]
#[serial]
fn array_input() {
    compile_and_execute("flowrcli", "array-input", true).expect("Test failed");
}

#[test]
#[serial]
fn double_connection() {
    compile_and_execute("flowrcli", "double-connection", true).expect("Test failed");
}

#[test]
#[serial]
fn two_destinations() {
    compile_and_execute("flowrcli", "two-destinations", true).expect("Test failed");
}

#[test]
#[serial]
fn flowc_hello_world() {
    compile_and_execute("flowrcli", "hello-world", true).expect("Test failed");
}

#[test]
#[serial]
fn doesnt_create_if_not_exist() {
    let non_existent_test = "__nope";
    let root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let non_existent = root_dir.join("tests/test-flows").join(non_existent_test);
    assert!(!non_existent.exists());
    assert!(compile_and_execute("flowrcli", non_existent_test, true).is_err());
    assert!(!non_existent.exists(), "File {non_existent_test} should not have been created");
}

#[test]
#[serial]
fn fibonacci_flowrgui() {
    compile_and_execute("flowrcli", "fibonacci", true).expect("Test failed");
}

#[test]
#[serial]
fn hello_world_client_server() {
    let manifest = compile_and_execute("flowrcli", "hello-world", false)
        .expect("Test failed");

    execute_flow_client_server("hello-world", manifest)
        .expect("Client/Server execution failed");
}

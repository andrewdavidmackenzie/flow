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

/*
fn execute_flow(
    filepath: PathBuf,
    test_args: Vec<String>,
    input: String,
    separate_processes: bool,
) -> (String, String) {
    let server = if separate_processes {
        let mut server_command = Command::new("flowr");
        // separate 'flowr' server process args: -n for native libs, -s to get a server process
        let server_command_args = vec!["-n", "-s"];

        println!("Starting 'flowr' with command line: 'flowr {}'",
                server_command_args.join(" "));

        // spawn the 'flowr' server process
        Some(
            server_command
                .args(server_command_args)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("Failed to spawn flowr"),
        )
    } else {
        None
    };

    let mut command = Command::new("flowr");
    let mut command_args = if separate_processes {
        // separate 'flowr' client process args
        vec!["-c"]
    } else {
        // when running client_and_server in same process we want to use native libs
        vec!["-n"]
    };

    // If a server was started - then kill it
    if let Some(mut server_child) = server {
        println!("Killing 'flowr' server");
        server_child.kill().expect("Failed to kill server child process");
    }

    (output, err)
}

 */

/*
#[test]
#[serial]
fn hello_world_client_server() {
    execute_test("hello-world", true)
        .expect("Could not run flow");
}
*/

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

fn compile_and_execute(test_name: &str) {
    let root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let context_dir = root_dir.join("src/cli");
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
        .expect("Could not spawn flowc process");

    // read it's stderr
    let mut actual_stderr = String::new();
    let stderr = execution.stderr.expect("Could not get stderr");
    for line in BufReader::new(stderr).lines() {
        let _ = writeln!(actual_stderr, "{}", &line.expect("Could not read line"));
    }

    assert_eq!("", actual_stderr);

    // read it's stdout
    let mut actual_stdout = String::new();
    let stdout = execution.stdout.expect("Could not get stdout");
    for line in BufReader::new(stdout).lines() {
        let _ = writeln!(actual_stdout, "{}", &line.expect("Could not read line"));
    }

    let expected_stdout = read_file(&test_dir, "expected.stdout");
    assert_eq!(expected_stdout, actual_stdout);
}

#[cfg(feature = "debugger")]
#[test]
#[serial]
fn debug_print_args() {
    compile_and_execute("debug-print-args");
}

#[test]
#[serial]
fn print_args() {
    compile_and_execute("print-args");
}

#[test]
#[serial]
fn hello_world() {
    compile_and_execute("hello-world");
}

#[test]
#[serial]
fn line_echo() {
    compile_and_execute("line-echo");
}

#[test]
#[serial]
fn args() {
    compile_and_execute("args");
}

#[test]
#[serial]
fn args_json() {
    compile_and_execute("args-json");
}

#[test]
#[serial]
fn array_input() {
    compile_and_execute("array-input")
}

#[test]
#[serial]
fn double_connection() {
    compile_and_execute("double-connection");
}

#[test]
#[serial]
fn two_destinations() {
    compile_and_execute("two-destinations");
}

/*
#[test]
#[serial]
fn doesnt_create_if_not_exist() {
    let non_existent = dir.join("__nope");
    assert!(!non_existent.exists());

    compile_and_execute(&non_existent.to_string_lossy());

    assert!(!non_existent.exists(), "File {} was created and should not have been",
            non_existent.to_string_lossy());
}

 */

#[test]
#[serial]
fn flowc_hello_world() {
    compile_and_execute("hello-world");
}
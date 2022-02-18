#![deny(missing_docs)]
#![warn(clippy::unwrap_used)]

//! This is the `flowstdlib` standard library of functions for `flow` programs

/// We'll put our errors in an `errors` module, and other modules in this crate will `use errors::*;`
/// to get access to everything `error_chain` creates.
pub mod errors;

/// functions from module 'data'
pub mod data;

/// functions from module 'control'
pub mod control;

/// functions from module 'math'
pub mod math;

/// functions from module 'fmt'
pub mod fmt;

/// manifest
pub mod manifest;

#[cfg(test)]
pub mod test {
    use std::env;
    use std::fs::File;
    use std::io::{Read, Write};
    use std::path::PathBuf;
    use std::process::{Command, Stdio};

    use tempdir::TempDir;

    fn execute_flow(filepath: PathBuf) -> String {
        let mut command = Command::new("flowc");
        let command_args = vec![filepath.to_str().expect("Couldn't convert file path to string")];

        // spawn the 'flowc' child process
        let mut runner = command
            .args(command_args)
            .stdin(Stdio::inherit())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("Couldn't spawn flowc to run test flow");

        let result = runner.wait().expect("failed to wait on child");

        // read it's stdout
        let mut output = String::new();
        if let Some(ref mut stdout) = runner.stdout {
            stdout.read_to_string(&mut output).expect("Could not read stdout");
        }

        println!("stdout = {}", output);

        assert!(result.success());
        output
    }

    #[test]
    fn test_range_flow() {
        let flow = "\
flow = \"range_test\"

[[process]]
source = \"lib://flowstdlib/math/range\"
input.range = { once = [1, 10] }

[[process]]
source = \"lib://context/stdio/stdout\"

[[connection]]
from = \"range/number\"
to = \"stdout\"
";

        let temp_dir = TempDir::new("flow").expect("Could not create TempDir").into_path();
        let flow_filename = temp_dir.join("range_test.toml");
        let mut flow_file =
            File::create(&flow_filename).expect("Could not create lib manifest file");
        flow_file.write_all(flow.as_bytes()).expect("Could not write data bytes to created flow file");

        let cwd = env::current_dir().expect("Could not get the current working directory");
        println!("CWD = {}", cwd.display());

        let stdout = execute_flow(flow_filename);

        let mut numbers: Vec<i32> = stdout.lines().map(|l| l.parse::<i32>().expect("Not a number")).collect::<Vec<i32>>();
        numbers.sort_unstable();
        assert_eq!(numbers, vec!(1, 2, 3, 4, 5, 6, 7, 8, 9, 10));
    }
}
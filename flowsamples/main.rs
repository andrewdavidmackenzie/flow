use std::{env, fs, io};
// Run sample flows found in any sub-directory
use std::fs::File;
use std::io::{BufRead, BufReader, ErrorKind};
use std::path::Path;
use std::process::{Command, Stdio};

const STDOUT_FILENAME : &str = "test.stdout";
const STDIN_FILENAME : &str = "test.stdin";
const STDERR_FILENAME : &str = "test.stderr";
const FILE_FILENAME : &str = "test.file";
const ARGS_FILENAME : &str = "test.args";

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

fn run_sample(sample_dir: &Path, output_dir: &Path, flowrex: bool) -> io::Result<()> {
    let manifest_path = output_dir.join("manifest.json");
    println!("\n\tRunning Sample: {:?}", sample_dir.file_name());
    assert!(manifest_path.exists(), "Manifest not found at '{}'", manifest_path.display());
    println!("\tSTDIN is read from {STDIN_FILENAME}, Arguments are read from {ARGS_FILENAME}");
    println!("\tSTDOUT is sent to {STDOUT_FILENAME}, STDERR to {STDERR_FILENAME} and file output to {FILE_FILENAME}");

    let mut command_args: Vec<String> = vec!["--native".into()];

    if flowrex {
        command_args.push("--context".into())
    }

    command_args.push( manifest_path.display().to_string());

    command_args.append(&mut args(sample_dir)?);

    let output = File::create(output_dir.join(STDOUT_FILENAME))
        .expect("Could not get directory as string");
    let error = File::create(output_dir.join(STDERR_FILENAME))
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

    println!("Command line: 'flowr {}'", command_args.join(" "));
    match Command::new("flowr")
        .args(command_args)
        .current_dir(output_dir.canonicalize()?)
        .stdin(Stdio::piped())
        .stdout(Stdio::from(output))
        .stderr(Stdio::from(error))
        .spawn()
    {
        Ok(mut flowr_child) => {
            let stdin_file = sample_dir.join(STDIN_FILENAME);
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

fn args(sample_dir: &Path) -> io::Result<Vec<String>> {
    let args_file = sample_dir.join(ARGS_FILENAME);

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
    use serial_test::serial;

    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::{Command, Stdio};

    use crate::{FILE_FILENAME, STDERR_FILENAME, STDOUT_FILENAME};

    fn test_sample(name: &str, flowrex: bool) {
        let samples_root = env!("CARGO_MANIFEST_DIR");
        let samples_dir = Path::new(samples_root);
        let sample_dir = samples_dir.join(name);

        let root_dir = samples_dir.parent().expect("Could not get parent directory");
        let samples_out_dir = root_dir.join("target/flowsamples");
        let output_dir = samples_out_dir.join(name);

        // Remove any previous output
        let _ = fs::remove_file(output_dir.join(STDERR_FILENAME));
        let _ = fs::remove_file(output_dir.join(FILE_FILENAME));
        let _ = fs::remove_file(output_dir.join(STDOUT_FILENAME));

        super::run_sample(&sample_dir, &output_dir, flowrex)
            .expect("Running of test sample failed");

        check_test_output(&sample_dir, &output_dir);

        // if test passed, remove output
        let _ = fs::remove_file(output_dir.join(STDERR_FILENAME));
        let _ = fs::remove_file(output_dir.join(FILE_FILENAME));
        let _ = fs::remove_file(output_dir.join(STDOUT_FILENAME));
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

        compare_and_fail(sample_dir.join("expected.stdout"), output_dir.join(STDOUT_FILENAME));
        compare_and_fail(sample_dir.join("expected.file"), output_dir.join(FILE_FILENAME));
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

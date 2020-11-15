// Run sample flows found in any subfolder
use std::{env, fs, io};
use std::fs::File;
use std::io::{BufRead, BufReader, ErrorKind};
use std::path::Path;
use std::process::{Command, Stdio};

fn main() -> io::Result<()> {
    // Try the local 'flowr' in the development tree
    let mut flowr = Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/debug/flowr");
    if !flowr.exists() {
        // if it doesn't exist then let's try a 'flowr' the user has installed with 'cargo'
        flowr = Path::new("flowr").to_path_buf();
    }

    let args: Vec<String> = env::args().collect();

    match args.len() {
        1 => {
            // find all sample sub-folders below this crate root
            for entry in fs::read_dir(env!("CARGO_MANIFEST_DIR"))? {
                if let Ok(e) = entry {
                    if e.metadata()?.is_dir() {
                        run_sample(&e.path(), &flowr)?
                    }
                };
            }
        }
        2 => {
            let samples_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join(&args[1]);
            run_sample(&samples_dir, &flowr)?
        }
        _ => eprintln!("Usage: {} <optional_sample_directory_name>", args[0])
    }

    Ok(())
}

fn run_sample(sample_dir: &Path, flowr_path: &Path) -> io::Result<()> {
    // Remove any previous output
    let _ = fs::remove_file(sample_dir.join("test.err"));
    let _ = fs::remove_file(sample_dir.join("test.file"));
    let _ = fs::remove_file(sample_dir.join("test.output"));

    let mut flowr_command = Command::new(flowr_path);
    let manifest = sample_dir.join("manifest.json");
    println!("\tRunning Sample: {:?}", sample_dir.file_name().unwrap());
    println!("\tReading STDIN from test.input, Arguments read from test.arguments");
    println!("\tOutput sent to STDOUT/STDERR and file output to test.file");

    let mut command_args: Vec<String> = vec!("--native".into(), manifest.display().to_string());
    command_args.append(&mut args(&sample_dir)?);

    match flowr_command.args(command_args)
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn() {
        Ok(mut flowr_child) => {
            let _ = Command::new("cat")
                .args(vec!(sample_dir.join("test.input")))
                .stdout(flowr_child.stdin.take()
                    .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Could not take STDIN of `flowr` process"))?)
                .spawn().unwrap();

            match flowr_child.wait_with_output() {
                Ok(_) => Ok(()),
                Err(e) => Err(io::Error::new(io::ErrorKind::Other,
                                             format!("Error running `flowr`: {}", e)))
            }
        }
        Err(e) => {
            match e.kind() {
                ErrorKind::NotFound =>
                    Err(io::Error::new(io::ErrorKind::Other,
                                       format!("`flowr` was not found! Check your $PATH. {}", e))),
                _ => Err(io::Error::new(io::ErrorKind::Other,
                                        format!("Unexpected error occurred spawning `flowr`: {}", e)))
            }
        }
    }
}

fn args(sample_dir: &Path) -> io::Result<Vec<String>>  {
    let args_file = sample_dir.join("test.arguments");
    let f = File::open(&args_file)?;
    let f = BufReader::new(f);

    let mut args = Vec::new();
    for line in f.lines() {
        args.push(line?);
    }

    Ok(args)
}

#[cfg(test)]
mod test {
    use std::fs;
    use std::fs::File;
    use std::io::{BufRead, BufReader};
    use std::path::Path;
    use std::process::{Command, Stdio};

    fn test_run_sample(sample_dir: &Path, flowr_path: &Path) {
        // Remove any previous output
        let _ = fs::remove_file(sample_dir.join("test.err"));
        let _ = fs::remove_file(sample_dir.join("test.file"));
        let _ = fs::remove_file(sample_dir.join("test.output"));

        let mut flowr_command = Command::new(flowr_path);
        eprintln!("\tRunning Sample: {:?}", sample_dir.file_name().unwrap());

        let manifest = sample_dir.join("manifest.json");

        let mut command_args: Vec<String> = vec!("--native".into(), manifest.display().to_string());
        command_args.append(&mut super::args(&sample_dir).unwrap());

        let output = File::create(sample_dir.join("test.output")).unwrap();
        let error = File::create(sample_dir.join("test.err")).unwrap();
        let mut flowr_child = flowr_command.args(command_args)
            .stdin(Stdio::piped())
            .stdout(Stdio::from(output))
            .stderr(Stdio::from(error))
            .spawn().unwrap();

        let _ = Command::new("cat")
            .args(vec!(sample_dir.join("test.input")))
            .stdout(flowr_child.stdin.take().unwrap())
            .spawn().unwrap();

        flowr_child.wait_with_output().unwrap();

        check_test_output(sample_dir)
    }

    fn compare_and_fail(sample_dir: &Path, expected_name: &str, actual_name: &str) {
        let expected = sample_dir.join(expected_name);
        if expected.exists() {
            let actual = sample_dir.join(actual_name);
            let diff = Command::new("diff")
                .args(vec!(&expected, &actual))
                .stdin(Stdio::inherit())
                .stderr(Stdio::inherit())
                .stdout(Stdio::inherit())
                .spawn().unwrap();
            let output = diff.wait_with_output().unwrap();
            assert!(output.status.success(),
                    format!("Sample {:?} {:?} does not match {:?}",
                            sample_dir.file_name().unwrap(),
                            actual.file_name().unwrap(),
                            expected.file_name().unwrap()));
        }
    }

    fn check_test_output(sample_dir: &Path) {
        let error = sample_dir.join("test.err");
        if error.exists() {
            let f = File::open(&error).unwrap();
            let mut f = BufReader::new(f);
            let contents = f.fill_buf().unwrap();

            if !contents.is_empty() {
                eprintln!("Sample {:?} produced error output in {}", sample_dir.file_name().unwrap(), error.display());
                std::process::exit(-1);
            }
        }

        compare_and_fail(sample_dir, "expected.output", "test.output");
        compare_and_fail(sample_dir, "expected.file", "test.file");
    }

    #[test]
    fn test_args() {
        let flowr = Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/debug/flowr");
        let sample = Path::new(env!("CARGO_MANIFEST_DIR")).join("args");
        test_run_sample(&sample, &flowr);
    }

    #[test]
    fn test_arrays() {
        let flowr = Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/debug/flowr");
        let sample = Path::new(env!("CARGO_MANIFEST_DIR")).join("arrays");
        test_run_sample(&sample, &flowr);
    }

    #[test]
    fn test_factorial() {
        let flowr = Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/debug/flowr");
        let sample = Path::new(env!("CARGO_MANIFEST_DIR")).join("factorial");
        test_run_sample(&sample, &flowr);
    }

    #[test]
    fn test_fibonacci() {
        let flowr = Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/debug/flowr");
        let sample = Path::new(env!("CARGO_MANIFEST_DIR")).join("fibonacci");
        test_run_sample(&sample, &flowr);
    }

    #[test]
    fn test_hello_world() {
        let flowr = Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/debug/flowr");
        let sample = Path::new(env!("CARGO_MANIFEST_DIR")).join("hello-world");
        test_run_sample(&sample, &flowr);
    }

    #[test]
    fn test_mandlebrot() {
        let flowr = Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/debug/flowr");
        let sample = Path::new(env!("CARGO_MANIFEST_DIR")).join("mandlebrot");
        test_run_sample(&sample, &flowr);
    }

    #[test]
    fn test_matrix_mult() {
        let flowr = Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/debug/flowr");
        let sample = Path::new(env!("CARGO_MANIFEST_DIR")).join("matrix_mult");
        test_run_sample(&sample, &flowr);
    }

    #[test]
    fn test_pipeline() {
        let flowr = Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/debug/flowr");
        let sample = Path::new(env!("CARGO_MANIFEST_DIR")).join("pipeline");
        test_run_sample(&sample, &flowr);
    }

    #[test]
    fn test_prime() {
        let flowr = Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/debug/flowr");
        let sample = Path::new(env!("CARGO_MANIFEST_DIR")).join("prime");
        test_run_sample(&sample, &flowr);
    }

    #[test]
    fn test_primitives() {
        let flowr = Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/debug/flowr");
        let sample = Path::new(env!("CARGO_MANIFEST_DIR")).join("primitives");
        test_run_sample(&sample, &flowr);
    }

    #[test]
    fn test_range() {
        let flowr = Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/debug/flowr");
        let sample = Path::new(env!("CARGO_MANIFEST_DIR")).join("range");
        test_run_sample(&sample, &flowr);
    }

    #[test]
    fn test_range_of_ranges() {
        let flowr = Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/debug/flowr");
        let sample = Path::new(env!("CARGO_MANIFEST_DIR")).join("range-of-ranges");
        test_run_sample(&sample, &flowr);
    }

    #[test]
    fn test_reverse_echo() {
        let flowr = Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/debug/flowr");
        let sample = Path::new(env!("CARGO_MANIFEST_DIR")).join("reverse-echo");
        test_run_sample(&sample, &flowr);
    }

    #[test]
    fn test_router() {
        let flowr = Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/debug/flowr");
        let sample = Path::new(env!("CARGO_MANIFEST_DIR")).join("router");
        test_run_sample(&sample, &flowr);
    }

    #[test]
    fn test_tokenizer() {
        let flowr = Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/debug/flowr");
        let sample = Path::new(env!("CARGO_MANIFEST_DIR")).join("tokenizer");
        test_run_sample(&sample, &flowr);
    }

    #[test]
    #[ignore]
    fn test_all_samples() {
        let flowr = Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/debug/flowr");

        // find all sample sub-folders below this crate root
        for entry in fs::read_dir(env!("CARGO_MANIFEST_DIR")).unwrap() {
            if let Ok(e) = entry {
                if e.metadata().unwrap().is_dir() {
                    test_run_sample(&e.path(), &flowr);
                }
            };
        }
    }
}
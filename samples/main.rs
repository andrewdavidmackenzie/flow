use std::{env, fs, io};
// Run sample flows found in any sub-directory
use std::fs::File;
use std::io::{BufRead, BufReader, ErrorKind};
use std::path::Path;
use std::process::{Command, Stdio};

use simpath::{FileType, Simpath};

fn main() -> io::Result<()> {
    println!("`flowsample` version {}", env!("CARGO_PKG_VERSION"));
    println!(
        "Current Working Directory: `{}`",
        std::env::current_dir().expect("Could not get working directory").display()
    );
    println!("Samples Root Directory: `{}`", env!("CARGO_MANIFEST_DIR"));

    let flowr = get_flowr()?;

    let args: Vec<String> = env::args().collect();

    match args.len() {
        1 => {
            // find all sample sub-folders below this crate root
            for entry in (fs::read_dir(env!("CARGO_MANIFEST_DIR"))?).flatten() {
                if entry.metadata()?.is_dir() {
                    run_sample(&entry.path(), &flowr)?
                }
            }
        }
        2 => {
            let samples_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join(&args[1]);
            run_sample(&samples_dir, &flowr)?
        }
        _ => eprintln!("Usage: {} <optional_sample_directory_name>", args[0]),
    }

    Ok(())
}

fn get_flowr() -> io::Result<String> {
    let dev = Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/debug/flowr");
    if dev.exists() {
        return Ok(dev.into_os_string().to_str().expect("Could not convert path to string").to_string());
    }

    let dev = Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/release/flowr");
    if dev.exists() {
        return Ok(dev.into_os_string().to_str().expect("Could not convert path to string").to_string());
    }

    if Simpath::new("PATH")
        .find_type("flowr", FileType::File)
        .is_ok()
    {
        return Ok("flowr".into());
    }

    Err(io::Error::new(
        io::ErrorKind::Other,
        "`flowr` could not be found in `$PATH` or `target/`",
    ))
}

fn run_sample(sample_dir: &Path, flowr_path: &str) -> io::Result<()> {
    // Remove any previous output
    let _ = fs::remove_file(sample_dir.join("test.err"));
    let _ = fs::remove_file(sample_dir.join("test.file"));
    let _ = fs::remove_file(sample_dir.join("test.output"));

    let mut flowr_command = Command::new(flowr_path);
    let manifest = sample_dir.join("manifest.json");
    println!("\n\tRunning Sample: {:?}", sample_dir.file_name());
    assert!(manifest.exists(), "Manifest file does not exist");
    println!("\tReading STDIN from test.input, Arguments read from test.arguments");
    println!("\tOutput sent to STDOUT/STDERR and file output to test.file");

    let mut command_args: Vec<String> = vec!["--native".into(), manifest.display().to_string()];
    command_args.append(&mut args(sample_dir)?);

    match flowr_command
        .args(command_args)
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
    {
        Ok(mut flowr_child) => {
            let _ = Command::new("cat")
                .args(vec![sample_dir.join("test.input")])
                .stdout(flowr_child.stdin.take().ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::Other,
                        "Could not take STDIN of `flowr` process",
                    )
                })?)
                .spawn();

            flowr_child.wait_with_output()?;
            Ok(())
        }
        Err(e) => match e.kind() {
            ErrorKind::NotFound => Err(io::Error::new(
                io::ErrorKind::Other,
                format!("`flowr` was not found! Check your $PATH. {}", e),
            )),
            _ => Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Unexpected error occurred spawning `flowr`: {}", e),
            )),
        },
    }
}

fn args(sample_dir: &Path) -> io::Result<Vec<String>> {
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

    use serial_test::serial;

    fn test_run_sample(name: &str) {
        let flowr =  &super::get_flowr().expect("Could not get flowr");
        let sample_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join(name);

        // Remove any previous output
        let _ = fs::remove_file(sample_dir.join("test.err"));
        let _ = fs::remove_file(sample_dir.join("test.file"));
        let _ = fs::remove_file(sample_dir.join("test.output"));

        let mut flowr_command = Command::new(flowr);
        println!("\tSample: {:?}", sample_dir.file_name().expect("Could no tget directory as string"));

        let manifest = sample_dir.join("manifest.json");
        assert!(manifest.exists(), "manifest.json does not exist");
        let mut command_args: Vec<String> = vec!["--native".into(), manifest.display().to_string()];
        command_args.append(&mut super::args(&sample_dir).expect("Could not get args"));

        let output = File::create(sample_dir.join("test.output")).expect("Could no tget directory as string");
        let error = File::create(sample_dir.join("test.err")).expect("Could no tget directory as string");
        let mut flowr_child = flowr_command
            .args(command_args)
            .stdin(Stdio::piped())
            .stdout(Stdio::from(output))
            .stderr(Stdio::from(error))
            .spawn()
            .expect("Could not spawn child process");

        let _ = Command::new("cat")
            .args(vec![sample_dir.join("test.input")])
            .stdout(flowr_child.stdin.take().expect("Could not get stdin"))
            .spawn()
            .expect("Could not spawn cat");

        flowr_child.wait_with_output().expect("Could not get child process output");

        check_test_output(&sample_dir);
    }

    fn compare_and_fail(sample_dir: &Path, expected_name: &str, actual_name: &str) {
        let expected = sample_dir.join(expected_name);
        if expected.exists() {
            let actual = sample_dir.join(actual_name);
            let diff = Command::new("diff")
                .args(vec![&expected, &actual])
                .stdin(Stdio::inherit())
                .stderr(Stdio::inherit())
                .stdout(Stdio::inherit())
                .spawn()
                .expect("Could not get child process");
            let output = diff.wait_with_output().expect("Could not get child process output");
            assert!(
                output.status.success(),
                "Output doesn't match the expected output"
            );
        }
    }

    fn check_test_output(sample_dir: &Path) {
        let error = sample_dir.join("test.err");
        if error.exists() {
            let f = File::open(&error).expect("Could not open file");
            let mut f = BufReader::new(f);
            let contents = f.fill_buf().expect("Could not read from file");

            if !contents.is_empty() {
                eprintln!(
                    "Sample {:?} produced error output in {}",
                    sample_dir.file_name().expect("Could not get directory file name"),
                    error.display()
                );
                std::process::exit(1);
            }
        }

        compare_and_fail(sample_dir, "expected.output", "test.output");
        compare_and_fail(sample_dir, "expected.file", "test.file");
    }

    #[test]
    #[serial]
    fn test_args() {
        test_run_sample("args");
    }

    #[test]
    #[serial]
    fn test_arrays() {
        test_run_sample("arrays");
    }

    #[test]
    #[serial]
    fn test_factorial() {
        test_run_sample("factorial");
    }

    #[test]
    #[serial]
    fn test_fibonacci() {
        test_run_sample("fibonacci");
    }

    #[test]
    #[serial]
    fn test_hello_world() {
        test_run_sample("hello-world");
    }

    #[test]
    #[serial]
    fn test_matrix_multiplication_sample() {
        test_run_sample("matrix_mult");
    }

    #[test]
    #[serial]
    fn test_pipeline() {
        test_run_sample("pipeline");
    }

    #[test]
    #[serial]
    fn test_prime() {
        test_run_sample("prime");
    }

    #[test]
    #[serial]
    fn test_primitives() {
        test_run_sample("primitives");
    }

    #[test]
    #[serial]
    fn test_sequence() {
        test_run_sample("sequence");
    }

    #[test]
    #[serial]
    fn test_sequence_of_sequences() {
        test_run_sample("sequence-of-sequences");
    }

    #[test]
    #[serial]
    fn test_router() {
        test_run_sample("router");
    }

    #[test]
    #[serial]
    fn test_tokenizer() {
        test_run_sample("tokenizer");
    }

    #[test]
    #[serial]
    fn test_reverse_echo() {
        test_run_sample("reverse-echo");
    }

    // This is the only sample that is using provided implementations and hence is executing WASM
    #[ignore]
    #[test]
    #[serial]
    fn test_mandlebrot() {
        test_run_sample("mandlebrot");
    }
}

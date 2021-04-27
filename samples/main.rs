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
        std::env::current_dir().unwrap().display()
    );
    println!("Samples Root Directory: `{}`", env!("CARGO_MANIFEST_DIR"));

    let flowr = get_flowr()?;

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
        _ => eprintln!("Usage: {} <optional_sample_directory_name>", args[0]),
    }

    Ok(())
}

fn get_flowr() -> io::Result<String> {
    let dev = Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/debug/flowr");
    if dev.exists() {
        return Ok(dev.into_os_string().to_str().unwrap().to_string());
    }

    let dev = Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/release/flowr");
    if dev.exists() {
        return Ok(dev.into_os_string().to_str().unwrap().to_string());
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
    command_args.append(&mut args(&sample_dir)?);

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

            match flowr_child.wait_with_output() {
                Ok(_) => Ok(()),
                Err(e) => Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("Error running `flowr`: {}", e),
                )),
            }
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

    fn test_run_sample(sample_dir: &Path, flowr: &str) {
        // Remove any previous output
        let _ = fs::remove_file(sample_dir.join("test.err"));
        let _ = fs::remove_file(sample_dir.join("test.file"));
        let _ = fs::remove_file(sample_dir.join("test.output"));

        let mut flowr_command = Command::new(flowr);
        println!("\tSample: {:?}", sample_dir.file_name().unwrap());

        let manifest = sample_dir.join("manifest.json");
        assert!(manifest.exists(), "manifest.json does not exist");
        let mut command_args: Vec<String> = vec!["--native".into(), manifest.display().to_string()];
        command_args.append(&mut super::args(&sample_dir).unwrap());

        let output = File::create(sample_dir.join("test.output")).unwrap();
        let error = File::create(sample_dir.join("test.err")).unwrap();
        let mut flowr_child = flowr_command
            .args(command_args)
            .stdin(Stdio::piped())
            .stdout(Stdio::from(output))
            .stderr(Stdio::from(error))
            .spawn()
            .unwrap();

        let _ = Command::new("cat")
            .args(vec![sample_dir.join("test.input")])
            .stdout(flowr_child.stdin.take().unwrap())
            .spawn()
            .unwrap();

        flowr_child.wait_with_output().unwrap();

        check_test_output(sample_dir)
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
                .unwrap();
            let output = diff.wait_with_output().unwrap();
            assert!(
                output.status.success(),
                "Output doesn't match the expected output"
            );
        }
    }

    fn check_test_output(sample_dir: &Path) {
        let error = sample_dir.join("test.err");
        if error.exists() {
            let f = File::open(&error).unwrap();
            let mut f = BufReader::new(f);
            let contents = f.fill_buf().unwrap();

            if !contents.is_empty() {
                eprintln!(
                    "Sample {:?} produced error output in {}",
                    sample_dir.file_name().unwrap(),
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
        let sample = Path::new(env!("CARGO_MANIFEST_DIR")).join("args");
        test_run_sample(&sample, &super::get_flowr().unwrap());
    }

    #[test]
    #[serial]
    fn test_arrays() {
        let sample = Path::new(env!("CARGO_MANIFEST_DIR")).join("arrays");
        test_run_sample(&sample, &super::get_flowr().unwrap());
    }

    #[test]
    #[serial]
    fn test_factorial() {
        let sample = Path::new(env!("CARGO_MANIFEST_DIR")).join("factorial");
        test_run_sample(&sample, &super::get_flowr().unwrap());
    }

    #[test]
    #[serial]
    fn test_fibonacci() {
        let sample = Path::new(env!("CARGO_MANIFEST_DIR")).join("fibonacci");
        test_run_sample(&sample, &super::get_flowr().unwrap());
    }

    #[test]
    #[serial]
    fn test_hello_world() {
        let sample = Path::new(env!("CARGO_MANIFEST_DIR")).join("hello-world");
        test_run_sample(&sample, &super::get_flowr().unwrap());
    }

    #[test]
    #[serial]
    fn test_mandlebrot() {
        let sample = Path::new(env!("CARGO_MANIFEST_DIR")).join("mandlebrot");
        test_run_sample(&sample, &super::get_flowr().unwrap());
    }

    #[test]
    #[serial]
    fn test_matrix_multiplication_sample() {
        let sample = Path::new(env!("CARGO_MANIFEST_DIR")).join("matrix_mult");
        test_run_sample(&sample, &super::get_flowr().unwrap());
    }

    #[test]
    #[serial]
    fn test_pipeline() {
        let sample = Path::new(env!("CARGO_MANIFEST_DIR")).join("pipeline");
        test_run_sample(&sample, &super::get_flowr().unwrap());
    }

    #[test]
    #[serial]
    fn test_prime() {
        let sample = Path::new(env!("CARGO_MANIFEST_DIR")).join("prime");
        test_run_sample(&sample, &super::get_flowr().unwrap());
    }

    #[test]
    #[serial]
    fn test_primitives() {
        let sample = Path::new(env!("CARGO_MANIFEST_DIR")).join("primitives");
        test_run_sample(&sample, &super::get_flowr().unwrap());
    }

    #[test]
    #[serial]
    fn test_range() {
        let sample = Path::new(env!("CARGO_MANIFEST_DIR")).join("range");
        test_run_sample(&sample, &super::get_flowr().unwrap());
    }

    #[test]
    #[serial]
    fn test_range_of_ranges() {
        let sample = Path::new(env!("CARGO_MANIFEST_DIR")).join("range-of-ranges");
        test_run_sample(&sample, &super::get_flowr().unwrap());
    }

    #[test]
    #[serial]
    fn test_reverse_echo() {
        let sample = Path::new(env!("CARGO_MANIFEST_DIR")).join("reverse-echo");
        test_run_sample(&sample, &super::get_flowr().unwrap());
    }

    #[test]
    #[serial]
    fn test_router() {
        let sample = Path::new(env!("CARGO_MANIFEST_DIR")).join("router");
        test_run_sample(&sample, &super::get_flowr().unwrap());
    }

    #[test]
    #[serial]
    fn test_tokenizer() {
        let sample = Path::new(env!("CARGO_MANIFEST_DIR")).join("tokenizer");
        test_run_sample(&sample, &super::get_flowr().unwrap());
    }

    // #[test]
    // #[ignore]
    // fn test_all_samples() {
    //     let flowr = super::get_flowr().unwrap();
    //
    //     // find all sample sub-folders below this crate root
    //     for entry in fs::read_dir(env!("CARGO_MANIFEST_DIR")).unwrap() {
    //         if let Ok(e) = entry {
    //             if e.metadata().unwrap().is_dir() {
    //                 test_run_sample(&e.path(), flowr);
    //             }
    //         };
    //     }
    // }
}

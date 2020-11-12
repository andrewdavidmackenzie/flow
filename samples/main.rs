// Run sample flows found in any subfolder
use std::{env, fs, io};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};

fn main() -> io::Result<()> {
    let flowr = Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/debug/flowr");

    let args: Vec<String> = env::args().collect();

    match args.len() {
        1 => {
            // find all sample sub-folders below this crate root
            for entry in fs::read_dir(env!("CARGO_MANIFEST_DIR"))? {
                if let Ok(e) = entry {
                    if e.metadata().unwrap().is_dir() {
                        run_sample(&e.path(), &flowr);
                    }
                };
            }
        },
        2 => {
            let samples_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join(&args[1]);
            run_sample(&samples_dir, &flowr)
        },
        _ => eprintln!("Usage: {} <optional_sample_directory_name>", args[0])
    }

    Ok(())
}

fn run_sample(sample_dir: &Path, flowr_path: &Path) {
    let mut flowr_command = Command::new(flowr_path);
    let manifest = sample_dir.join("manifest.json");
    println!("\tRunning Sample: {:?}", sample_dir.file_name().unwrap());
    println!("\tReading STDIN from test.input, Arguments read from test.arguments");
    println!("\tOutput sent to STDOUT/STDERR and file output to test.file");

    let mut command_args: Vec<String> = vec!("--native".into(), manifest.display().to_string());
    command_args.append(&mut args(&sample_dir));

    let mut flowr_child = flowr_command.args(command_args)
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn().unwrap();

    let _ = Command::new("cat")
        .args(vec!(sample_dir.join("test.input")))
        .stdout(flowr_child.stdin.take().unwrap())
        .spawn().unwrap();

    flowr_child.wait_with_output().unwrap();
}

fn args(sample_dir: &Path) -> Vec<String> {
    let args_file = sample_dir.join("test.arguments");
    let f = File::open(&args_file).unwrap();
    let f = BufReader::new(f);

    let mut args = Vec::new();
    for line in f.lines() {
        args.push(line.unwrap());
    }
    args
}

#[cfg(test)]
mod test {
    use std::fs::File;
    use std::path::Path;
    use std::process::{Command, Stdio};

    fn test_run_sample(sample_dir: &Path, flowr_path: &Path) {
        let mut flowr_command = Command::new(flowr_path);
        let manifest = sample_dir.join("manifest.json");
        let output = File::create(sample_dir.join("test.output")).unwrap();
        let error = File::create(sample_dir.join("test.err")).unwrap();
        println!("\tRunning Sample: {}", manifest.display());

        let mut command_args: Vec<String> = vec!("--native".into(), manifest.display().to_string());
        command_args.append(&mut super::args(&sample_dir));

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
    }
}
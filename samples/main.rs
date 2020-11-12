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

// @cat $< | RUST_BACKTRACE=1 cargo run --quiet -p flowr -- --native $(@D) `cat $(@D)/test.arguments` 2> $(@D)/test.err > $@
// @diff $@ $(@D)/expected.output || (ret=$$?; cp $@ $(@D)/failed.output && rm -f $@ && rm -f $(@D)/test.file && exit $$ret)
// @if [ -s $(@D)/expected.file ]; then diff $(@D)/expected.file $(@D)/test.file; fi;
// @if [ -s $(@D)/test.err ]; then (printf " has error output in $(@D)/test.err\n"; exit -1); else printf " has no errors\n"; fi;
// @rm $@ #remove test.output after successful diff so that dependency will cause it to run again next time
// # leave test.err for inspection in case of failure
fn run_sample(sample_dir: &Path, flowr_path: &Path) {
    let mut flowr_command = Command::new(flowr_path);
    let manifest = sample_dir.join("manifest.json");
    let output = File::create(sample_dir.join("test.output")).unwrap();
    let error = File::create(sample_dir.join("test.err")).unwrap();
    println!("\tRunning Sample: {}", manifest.display());

    let mut command_args: Vec<String> = vec!("--native".into(), manifest.display().to_string());
    command_args.append(&mut args(&sample_dir));

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
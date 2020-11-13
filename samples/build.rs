// Build script to compile the flow samples in the crate

use std::{fs, io};
use std::path::Path;
use std::process::{Command, Stdio};

fn main() -> io::Result<()> {
    // find all sample sub-folders
    fs::read_dir(".")?
        .map(|res| res.map(|e| {
            if e.metadata().unwrap().is_dir() {
                compile_sample(&e.path());
            }
        }))
        .collect::<Result<Vec<_>, io::Error>>()?;

    println!("cargo:rerun-if-env-changed=FLOW_LIB_PATH");

    Ok(())
}

fn compile_sample(sample_dir: &Path) {
    // Tell Cargo that if the given file changes, to rerun this build script.
    println!("cargo:rerun-if-changed={}/context.toml", sample_dir.display());

    let mut command = Command::new("../target/debug/flowc");
    // -g for debug symbols, -d to dump compiler structs, -s to skip running, only compile the flow
    let command_args = vec!("-g", "-d", "-s", sample_dir.to_str().unwrap());

    command.args(command_args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn().unwrap();
}

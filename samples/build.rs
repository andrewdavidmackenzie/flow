// Build script to compile the flow samples in the crate
use std::{fs, io};
use std::io::ErrorKind;
use std::path::Path;
use std::process::{Command, Stdio};

use simpath::{FileType, Simpath};

fn main() -> io::Result<()> {
    let flowc = if Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/debug/flowc").exists() {
        "../target/debug/flowc"
    } else if Simpath::new("PATH").find_type("flowc", FileType::File).is_ok() {
        "flowc"
    } else {
        ""
    };

    if flowc.is_empty() {
        Err(io::Error::new(io::ErrorKind::Other, "Could not find `flowc` so `flowsamples` cannot be built"))
    } else {
        // find all sample sub-folders
        for entry in fs::read_dir(env!("CARGO_MANIFEST_DIR"))? {
            if let Ok(e) = entry {
                if let Ok(ft) = e.file_type() {
                    if ft.is_dir() {
                        compile_sample(&e.path(), flowc)?;
                    }
                }
            }
        };

        println!("cargo:rerun-if-env-changed=FLOW_LIB_PATH");
        Ok(())
    }
}

fn compile_sample(sample_dir: &Path, flowc: &str) -> io::Result<()> {
    // Tell Cargo that if the given file changes, to rerun this build script.
    println!("cargo:rerun-if-changed={}/context.toml", sample_dir.display());

    let mut command = Command::new(flowc);
    // -g for debug symbols, -d to dump compiler structs, -s to skip running, only compile the flow
    // let command_args = vec!("-g", "-d", "-s", sample_dir.to_str().unwrap());
    let command_args = vec!("-g", "-s", sample_dir.to_str().unwrap());

    match command.args(command_args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn() {
        Ok(flowc_child) => {
            match flowc_child.wait_with_output() {
                Ok(_) => Ok(()),
                Err(e) => Err(io::Error::new(io::ErrorKind::Other,
                                             format!("Error running `flowc`: {}", e)))
            }
        }
        Err(e) => {
            match e.kind() {
                ErrorKind::NotFound =>
                    Err(io::Error::new(io::ErrorKind::Other,
                                       format!("`flowc` was not found! Check your $PATH. {}", e))),
                _ => Err(io::Error::new(io::ErrorKind::Other,
                                        format!("Unexpected error occurred spawning `flowc`: {}", e)))
            }
        }
    }
}

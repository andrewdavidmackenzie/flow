use std::{fs, io};
// Build script to compile the flow samples in the crate
use std::io::ErrorKind;
use std::path::Path;
use std::process::{Command, Stdio};

use simpath::{FileType, Simpath};

fn main() -> io::Result<()> {
    let flowc = get_flowc()?;

    // find all sample sub-folders
    for entry in fs::read_dir(env!("CARGO_MANIFEST_DIR"))? {
        if let Ok(e) = entry {
            if let Ok(ft) = e.file_type() {
                if ft.is_dir() {
                    compile_sample(&e.path(), &flowc)?;
                }
            }
        }
    }

    println!("cargo:rerun-if-env-changed=FLOW_LIB_PATH");
    Ok(())
}

fn get_flowc() -> io::Result<String> {
    let dev = Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/debug/flowc");
    if dev.exists() {
        return Ok(dev.into_os_string().to_str().unwrap().to_string());
    }

    let dev = Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/release/flowc");
    if dev.exists() {
        return Ok(dev.into_os_string().to_str().unwrap().to_string());
    }

    if Simpath::new("PATH")
        .find_type("flowr", FileType::File)
        .is_ok()
    {
        return Ok("flowc".into());
    }

    Err(io::Error::new(
        io::ErrorKind::Other,
        "`flowc` could not be found in `$PATH` or `target/`",
    ))
}

fn compile_sample(sample_dir: &Path, flowc: &str) -> io::Result<()> {
    // Tell Cargo that if any file in the sample directory changes it should rerun this build script
    println!("cargo:rerun-if-changed={}", sample_dir.display());

    let mut command = Command::new(flowc);
    // -g for debug symbols, -d to dump compiler structs, -s to skip running, only compile the flow
    let command_args = vec!["-g", "-d", "-s", sample_dir.to_str().unwrap()];

    match command
        .args(command_args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
    {
        Ok(flowc_child) => match flowc_child.wait_with_output() {
            Ok(_) => Ok(()),
            Err(e) => Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Error running `flowc`: {}", e),
            )),
        },
        Err(e) => match e.kind() {
            ErrorKind::NotFound => Err(io::Error::new(
                io::ErrorKind::Other,
                format!("`flowc` was not found! Check your $PATH. {}", e),
            )),
            _ => Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Unexpected error occurred spawning `flowc`: {}", e),
            )),
        },
    }
}

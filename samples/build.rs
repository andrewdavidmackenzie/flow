//! Build script to compile the flow samples in the crate
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::{fs, io};

use glob::{glob_with, MatchOptions};
use simpath::{FileType, FoundType, Simpath};

#[allow(clippy::collapsible_if)]
fn main() -> io::Result<()> {
    let samples_root = env!("CARGO_MANIFEST_DIR");

    println!("cargo:rerun-if-env-changed=FLOW_LIB_PATH");
    // Tell Cargo that if any file in the samples directory changes it should rerun this build script
    println!("cargo:rerun-if-changed={}", samples_root);

    println!("`flowsample` version {}", env!("CARGO_PKG_VERSION"));
    println!(
        "Current Working Directory: `{}`",
        std::env::current_dir().unwrap().display()
    );
    println!("Samples Root Directory: `{}`", env!("CARGO_MANIFEST_DIR"));

    let flowc = get_flowc()?;

    println!(
        "Using 'flowc' compiler found at: `{}`",
        flowc.to_str().unwrap()
    );

    // find all sample sub-folders
    for entry in fs::read_dir(samples_root)? {
        let e = entry?;
        if e.file_type()?.is_dir() {
            println!(
                "\nBuilding sample in directory: `{}`",
                e.path().to_str().unwrap()
            );
            if compile_sample(&e.path(), &flowc).is_err() {
                std::process::exit(1);
            }
        }
    }

    std::process::exit(1);
    // Ok(())
}

fn get_flowc() -> io::Result<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();

    let dev = root.join("target/debug/flowc");
    if dev.exists() {
        return Ok(dev);
    }

    let dev = root.join("target/release/flowc");
    if dev.exists() {
        return Ok(dev);
    }

    if let Ok(FoundType::File(flowc)) = Simpath::new("PATH").find_type("flowc", FileType::File) {
        return Ok(flowc);
    }

    Err(io::Error::new(
        io::ErrorKind::Other,
        "`flowc` could not be found in `$PATH` or `target/`",
    ))
}

fn compile_sample(sample_dir: &Path, flowc: &Path) -> io::Result<()> {
    let mut command = Command::new(flowc);
    // -g for debug symbols, -z to dump graphs, -s to skip running and only compile the flow
    let command_args = vec!["-g", "-z", "-s", sample_dir.to_str().unwrap()];

    let flowc_child = command
        .args(command_args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()?;

    let flowc_output = flowc_child.wait_with_output()?;

    match flowc_output.status.code() {
        Some(0) => {}
        Some(_) => {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "`flowc` exited with non-zero status code",
            ))
        }
        _ => {}
    }

    generate_svgs(sample_dir)?;

    Ok(())
}

/*
   Generate SVG files from the .dot files created by flowc
*/
fn generate_svgs(sample_dir: &Path) -> io::Result<()> {
    if let Ok(FoundType::File(dot)) = Simpath::new("PATH").find_type("dot", FileType::File) {
        println!("Generating .dot.svg files from .dot files, using 'dot' command from $PATH");

        let mut dot_command = Command::new(dot);
        let options = MatchOptions {
            case_sensitive: false,
            ..Default::default()
        };
        let pattern = format!("{}/*.dot", sample_dir.to_str().unwrap());

        for entry in glob_with(&pattern, &options).unwrap() {
            if let Ok(path) = entry {
                let dot_child = dot_command
                    .args(vec!["-Tsvg", "-O", &path.to_str().unwrap()])
                    .stdin(Stdio::inherit())
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit())
                    .spawn()?;

                let dot_output = dot_child.wait_with_output()?;
                match dot_output.status.code() {
                    Some(0) => {}
                    Some(_) => {
                        return Err(io::Error::new(
                            io::ErrorKind::Other,
                            "`dot` exited with non-zero status code",
                        ))
                    }
                    _ => {}
                }
            }
        }
    } else {
        println!("Could not find 'dot' command in $PATH so SVG generation skipped");
    }

    Ok(())
}

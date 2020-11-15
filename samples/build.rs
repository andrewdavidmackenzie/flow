// Build script to compile the flow samples in the crate

use std::{fs, io};
use std::path::Path;
use std::process::{Command, Stdio};

// use simpath::{FileType, Simpath};

fn main() {
    // let flowc = if Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/debug/flowc").exists() {
    //     "../target/debug/flowc"
    // } else if Simpath::new("PATH").find_type("flowc", FileType::File).is_ok() {
    //     "flowc"
    // } else {
    //     ""
    // };
    //
    // if flowc.is_empty() {
    //     println!("cargo:warning=Could not find `flowc` in $PATH or `target/debug`, so cannot build flowsamples");
    // } else
    if Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/debug/flowc").exists() {
        // find all sample sub-folders
        fs::read_dir(".").unwrap()
            .map(|res| res.map(|e| {
                if e.metadata().unwrap().is_dir() {
                    compile_sample(&e.path(), "../target/debug/flowc");
                }
            }))
            .collect::<Result<Vec<_>, io::Error>>().unwrap();

        println!("cargo:rerun-if-env-changed=FLOW_LIB_PATH");
    }
}

fn compile_sample(sample_dir: &Path, flowc: &str) {
    // Tell Cargo that if the given file changes, to rerun this build script.
    println!("cargo:rerun-if-changed={}/context.toml", sample_dir.display());

    let mut command = Command::new(flowc);
    // -g for debug symbols, -d to dump compiler structs, -s to skip running, only compile the flow
    let command_args = vec!("-g", "-d", "-s", sample_dir.to_str().unwrap());

    command.args(command_args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn().unwrap();
}

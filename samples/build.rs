// Build script to compile the flow samples in the crate

use std::{fs, io};
use std::path::Path;
use std::process::{Command, Stdio};

fn main() {
    let mut flowc = Path::new(env!("CARGO_MANIFEST_DIR")).join("flowc");
    if !flowc.exists() {
        flowc = Path::new("flowc").to_path_buf();
    }

    if flowc.exists() {
        // find all sample sub-folders
        fs::read_dir(".").unwrap()
            .map(|res| res.map(|e| {
                if e.metadata().unwrap().is_dir() {
                    compile_sample(&e.path(), &flowc);
                }
            }))
            .collect::<Result<Vec<_>, io::Error>>().unwrap();

        println!("cargo:rerun-if-env-changed=FLOW_LIB_PATH");
    }
}

fn compile_sample(sample_dir: &Path, flowc: &Path) {
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

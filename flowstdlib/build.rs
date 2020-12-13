use std::path::Path;
use std::process::{Command, Stdio};

use simpath::{FileType, Simpath};

// Build script to compile the flowstdlib WASM files and generate manifest - using flowc
fn main() {
    let flowc = if Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/debug/flowc").exists() {
        "../target/debug/flowc"
    } else if Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/release/flowc").exists() {
        "../target/release/flowc"
    } else if Simpath::new("PATH").find_type("flowc", FileType::File).is_ok() {
        "flowc"
    } else {
        ""
    };

    if flowc.is_empty() {
        println!("cargo:warning=Could not find `flowc` in $PATH or `target/`, so cannot build flowstdlib");
    } else {
        let mut command = Command::new(flowc);
        // Options for flowc: -g for debug symbols, -d to dump compiler structs, -l for a library build
        let command_args = vec!("-v", "info", "-g", "-d", "-l", env!("CARGO_MANIFEST_DIR"));

        command.args(command_args)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn().unwrap();
    }

    check_flow_lib_path();
}

fn check_flow_lib_path() {
    let parent = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().display().to_string();
    match std::env::var("FLOW_LIB_PATH") {
        Err(_) => {
            println!("cargo:warning='FLOW_LIB_PATH' is not set, so 'flowstdlib' will not be found by 'flowc' or 'flowr'. Set it to an appropriate value thus: export FLOW_LIB_PATH=\"{}\"", parent);
        }
        Ok(value) => {
            let lib_path = Simpath::new("FLOW_LIB_PATH");
            if !lib_path.contains(&parent) {
                println!("cargo:warning='FLOW_LIB_PATH' is set to '{}'. But it does not contain the directory where 'flowstdlib' is, so 'flowstdlib' will not be found by 'flowc' or 'flowr'. Add an entry for this directory thus: export FLOW_LIB_PATH=\"{}:$FLOW_LIB_PATH\"",
                         value, parent);
            }
        }
    }
}
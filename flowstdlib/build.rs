use std::path::Path;
use std::process::{Command, Stdio};

use simpath::{FileType, Simpath};

use lib_path::check_flow_lib_path;

mod lib_path;

// Build script to compile the flowstdlib WASM files and generate manifest - using flowc
fn main() {
    let flowc = if Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../target/debug/flowc")
        .exists()
    {
        "../target/debug/flowc"
    } else if Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../target/release/flowc")
        .exists()
    {
        "../target/release/flowc"
    } else if Simpath::new("PATH")
        .find_type("flowc", FileType::File)
        .is_ok()
    {
        "flowc"
    } else {
        ""
    };

    if flowc.is_empty() {
        println!("cargo:warning=Could not find `flowc` in $PATH or `target/`, so cannot build flowstdlib");
    } else {
        let mut command = Command::new(flowc);
        // Options for flowc: -g for debug symbols, -d to dump compiler structs, -l for a library build
        let command_args = vec!["-v", "info", "-g", "-d", "-l", env!("CARGO_MANIFEST_DIR")];

        command
            .args(command_args)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
    }

    check_flow_lib_path();
}

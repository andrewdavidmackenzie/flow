use std::path::Path;
use std::process::{Command, Stdio};

// Build script to compile the flowstdlib WASM files and generate manifest - using flowc
fn main() {
    let flowc = Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/debug/flowc");

    if flowc.exists() {
        let mut command = Command::new(flowc);
        // let mut command = Command::new("../target/debug/flowc");
        // Options for flowc: -g for debug symbols, -d to dump compiler structs, -l for a library build
        let command_args = vec!("-v", "info", "-g", "-d", "-l", env!("CARGO_MANIFEST_DIR"));

        command.args(command_args)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn().unwrap();
    }
}
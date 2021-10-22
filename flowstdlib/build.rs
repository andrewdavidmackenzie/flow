use std::io;
use std::process::Command;

// Build script to compile the flowstdlib library (compile WASM files and generate manifest) using flowc
fn main() -> io::Result<()> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let out_dir = std::env::var("OUT_DIR")
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
    println!("cargo:warning=out_dir is {}", out_dir);

    // Tell Cargo that if any file changes it should rerun this build script
    println!("cargo:rerun-if-changed={}", manifest_dir);

    let mut command = Command::new("flowc");
    // Options for flowc:   -v info to give output,
    //                      -g for debug symbols
    //                      -z to dump graphs
    //                      -o to generate files in $out_dir
    //                      -l $dir to build library found in $manifest_dir

    let command_args = vec!["-v", "info", "-g", "-z", "-o", &out_dir, "-l", manifest_dir];
    //let command_args = vec!["-v", "info", "-g", "-z", "-l", manifest_dir];
    println!("Running command: flowc {:?}", command_args);

    let _ = command.args(command_args).output()?;

    Ok(())
}

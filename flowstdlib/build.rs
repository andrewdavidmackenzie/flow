use std::env;
use std::io;
use std::path::Path;
use std::process::{Command, Stdio};

// Build script to compile the flowstdlib library (compile WASM files and generate manifest) using flowc
fn main() -> io::Result<()> {
    let lib_root_dir = env!("CARGO_MANIFEST_DIR");
    let root_dir = Path::new(lib_root_dir).parent().expect("Could not get parent directory");
    let flowstdlib_out_dir = root_dir.join("target/flowstdlib");
    let out_dir = flowstdlib_out_dir.to_str().expect("Could not convert to str");

    // Tell Cargo that if any file changes it should rerun this build script
    println!("cargo:rerun-if-changed={}", lib_root_dir);

    let mut command = Command::new("flowc");
    // Options for flowc:   -v info : to log output at INFO level,
    //                      -n      : only build native implementations and not compile WASM files
    //                      -d      : to generate debug symbols in some output files (e.g. manifest.json)
    //                      -g      : to dump 'dot' graphs for documentation
    //                      -o      : generate files in $out_dir instead of current working directory
    //                      -l $dir : build the flow library found in $dir

    // If the "wasm" feature is activated, then don't set "-n" and flowc will compile implementations to wasm.
    #[cfg(feature = "wasm")]
    let command_args = vec!["-v", "info", "-d", "-g", "-l", "-o", out_dir, lib_root_dir];
    // If the "wasm" feature is NOT activated, then set "-n" (native only) flag so flowc will not compile to wasm
    #[cfg(not(feature = "wasm"))]
    let command_args = vec!["-v", "info", "-d", "-g", "-l", "-o", out_dir, "-n", lib_root_dir];

    let flowc_command = command
        .args(command_args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    let flowc_output = flowc_command.output()?;

    match flowc_output.status.code() {
        Some(0) | None => {}
        Some(_) => {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "`flowc` exited with non-zero status code",
            ))
        }
    }

    Ok(())
}

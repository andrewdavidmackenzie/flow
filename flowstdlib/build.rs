use std::env;
use std::io;
use std::process::{Command, Stdio};

// Build script to compile the flowstdlib library (compile WASM files and generate manifest) using flowc
fn main() -> io::Result<()> {
    let cwd = env::current_dir()?;
    let lib_root_dir = cwd
        .to_str()
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Couldn't get CWD"))?;
    let out_dir =
        env::var("OUT_DIR").map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

    // Tell Cargo that if any file changes it should rerun this build script
    println!("cargo:rerun-if-changed={}", lib_root_dir);

    let mut command = Command::new("flowc");
    // Options for flowc:   -v info to give output,
    //                      -n only build native implementations and skip WASM compile
    //                      -g for debug symbols
    //                      -z to dump graphs
    //                      -o to generate files in $out_dir
    //                      -l $dir to build library found in $manifest_dir


    // If the "wasm" is activated, then no "-n" will be set and flowc will also compile implementations to wasm.
    #[cfg(feature = "wasm")]
    let command_args = vec!["-v", "info", "-g", "-z", "-o", &out_dir, "-l", lib_root_dir];
    // If the "wasm" feature is NOT activated, then set "-n" (native only) flag so flowc will not compile to wasm
    #[cfg(not(feature = "wasm"))]
    let command_args = vec!["-v", "info", "-g", "-z", "-o", &out_dir, "-l", lib_root_dir, "-n"];

    println!("\tRunning command: flowc {}", command_args.join(" "));

    let flowc_child = command
        .args(command_args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()?;

    let flowc_output = flowc_child.wait_with_output()?;

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

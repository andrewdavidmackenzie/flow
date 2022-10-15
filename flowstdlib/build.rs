use std::env;
use std::io;
use std::path::Path;
use std::process::Command;

// Build script to compile the flowstdlib library (compile WASM files and generate manifest) using flowc
fn main() -> io::Result<()> {
    let lib_root_dir = env!("CARGO_MANIFEST_DIR");
    let workspace_root_dir = Path::new(lib_root_dir).parent().expect("Could not get parent directory");
    let flowstdlib_out_dir = workspace_root_dir.join("target/flowstdlib");
    let out_dir = flowstdlib_out_dir.to_str().expect("Could not convert to str");

    // Tell Cargo that if any file changes it should rerun this build script
    println!("cargo:rerun-if-changed={}", lib_root_dir);

    let mut command = Command::new("flowc");
    // flowc options:   -v info : to log output at INFO level,
    //                  -n      : only build native implementations and not compile WASM files
    //                  -d      : generate debug symbols in some output files (e.g. manifest.json)
    //                  -g      : dump 'dot' graphs for documentation
    //                  -O      : optimize the generated WASM output files
    //                  -o      : generate files in $out_dir instead of current working directory
    //                  -n      : do not compile to WASM, only compile a native version of the lib
    //                  -l      : compile a flow library (not a flow) who's path is the last arg

    // If the "wasm" feature is activated, then don't set "-n" and flowc will compile implementations to wasm.
    #[cfg(feature = "wasm")]
    let command_args = vec!["-d", "-g", "-l", "-O", "-o", out_dir, lib_root_dir];
    // If the "wasm" feature is NOT activated, then set "-n" (native only) flag so flowc will not compile to wasm
    #[cfg(not(feature = "wasm"))]
    let command_args = vec!["-d", "-g", "-l", "-o", out_dir, "-n", lib_root_dir];

    if !command
        .args(&command_args).status().expect("Could not get status").success() {
        eprintln!("Error building flowstdlib, command line\n flowc {}",
                  command_args.join(" "));
        std::process::exit(1);
    }

    Ok(())
}

use std::env;
use std::io;
use std::process::Command;

// Build script to compile the flowstdlib library (compile WASM files and generate manifest)
// using flowc
fn main() -> io::Result<()> {
    let lib_root_dir_str = env!("CARGO_MANIFEST_DIR");

    // Tell Cargo that if any file changes it should rerun this build script
    println!("cargo:rerun-if-changed={lib_root_dir_str}/src");

    let mut command = Command::new("flowc");
    // flowc options:   -v info : to log output at INFO level,
    //                  -n      : only build native implementations and not compile WASM files
    //                  -d      : generate debug symbols in some output files (e.g. manifest.json)
    //                  -g      : dump 'dot' graphs for documentation
    //                  -O      : optimize the generated WASM output files
    //                  -o      : generate files in $out_dir instead of current working directory
    //                  -n      : do not compile to WASM, only compile a native version of the lib
    //                  -l      : compile a flow library (not a flow) who's path is the last arg

//    let home_dir = env::var("HOME").expect("Could not get $HOME");
//    let out_dir = format!("{}/.flow/lib/flowstdlib", home_dir);
    let out_dir = env::var("OUT_DIR").unwrap();
    println!("cargo:warning=flowstdlib built in '{out_dir}'");
    let command_args = vec!["-d", "-g", "-l", "-O", "-o", &out_dir, lib_root_dir_str];

    match command.args(&command_args).status() {
        Ok(stat) => {
            if !stat.success() {
                eprintln!("Error building flowstdlib, command line\nflowc {}",
                          command_args.join(" "));
                std::process::exit(1);
            }
        }
        Err(err) => {
            eprintln!("Error building flowstdlib, command line\nflowc {}\nError: {}",
                      command_args.join(" "), err);
            std::process::exit(1);
        }
    }

    Ok(())
}

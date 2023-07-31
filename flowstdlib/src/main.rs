use std::env;
use std::io;
use std::process::Command;

// This binary compiles the flowstdlib library (compile WASM files and generate manifest)
// using flowc.
// It takes the root of the flowstdlib source directory as its only argument.
// It compiles the flowstdlib (wasm, docs etc) to $HOME/.flow/lib/flowstdlib
fn main() -> io::Result<()> {
    let mut command = Command::new("flowc");
    // flowc options:   -v info : to log output at INFO level,
    //                  -n      : only build native implementations and not compile WASM files
    //                  -d      : generate debug symbols in some output files (e.g. manifest.json)
    //                  -g      : dump 'dot' graphs for documentation
    //                  -O      : optimize the generated WASM output files
    //                  -o      : generate files in $out_dir instead of current working directory
    //                  -n      : do not compile to WASM, only compile a native version of the lib
    //                  -l      : compile a flow library (not a flow) who's path is the last arg

    let home_dir = env::var("HOME").expect("Could not get $HOME");
    let out_dir = format!("{}/.flow/lib/flowstdlib", home_dir);

    let lib_root_dir = env::args().nth(1).unwrap_or_else(
        || "No lib root directory specified. Please specify the directory where the flowstdlib \
        source resides".into()
    );

    let command_args = vec!["-d", "-v", "debug", "-g", "-l", "-O", "-o", &out_dir, &lib_root_dir];

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

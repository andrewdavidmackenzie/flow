use std::env;
use std::io;
use std::process::Command;

// This binary compiles the flowstdlib library (compile WASM files and generate manifest)
// using flowc.
// It takes the root of the flowstdlib source directory as its only argument.
// It compiles the flowstdlib (wasm, docs etc) to $HOME/.flow/lib/flowstdlib
fn main() -> io::Result<()> {
    let mut command = Command::new("flowc");
    // used flowc options:
    //   -d         : generate debug symbols in some output files (e.g. manifest.json)
    //   -g         : dump 'dot' graphs for documentation
    //   -L dir     : add 'dir' to the Lib Search path. This is so flows in the lib
    //                being compiled can reference flows or functions in the same
    //   -l         : compile a flow library (not a flow) who's path is the last arg
    //   -O         : optimize the generated WASM output files
    //   -v warn    : Set verbosity to warning level
    //   -o out_dir : generate files in $out_dir instead of current working directory
    //   source_dir : the source directory where the library resides

    let home_dir = env::var("HOME").expect("Could not get $HOME");
    let lib_home = format!("{}/.flow/lib", home_dir);
    let out_dir = format!("{}/flowstdlib", lib_home);

    let lib_source_dir = env::args().nth(1).expect("No lib root directory specified.\
     Please specify directory where flowstdlib source resides");

    let command_args = vec!["-d", "-g", "-L", &lib_home,  "-l", "-O",
                            "-v", "warn", "-o", &out_dir, &lib_source_dir];

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

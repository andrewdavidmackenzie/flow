//! Build script to compile the flow flowsamples in the crate
use std::{env, fs, io};
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() -> io::Result<()> {
    let flowsamples_root = env!("CARGO_MANIFEST_DIR");

    // if any file in the flowsamples directory changes, cargo should rerun this build script
    println!("cargo:rerun-if-changed={flowsamples_root}");

    let home_dir_str = env::var("HOME").expect("Could not get $HOME");
    let home_dir = Path::new(&home_dir_str);
    let samples_out_dir = home_dir.join(".flow/samples/flowsamples");

    // find all sample sub-folders at have a "root.toml" flow definition file and compile them
    for entry in fs::read_dir(flowsamples_root)? {
        let e = entry?;
        if e.file_type()?.is_dir() && e.path().join("root.toml").exists() {
            let sample_out_dir = &samples_out_dir.join(e.file_name());
            println!("Building sample '{}' to '{}'",
                     e.path().display(),
                     sample_out_dir.display());
            fs::create_dir_all(sample_out_dir).expect("Could not create output dir");
            compile_sample(&e.path().to_string_lossy(),
                           &sample_out_dir.to_string_lossy());
            println!("cargo:rerun-if-changed={}", &sample_out_dir.to_string_lossy());
        }
    }

    Ok(())
}

fn get_context_root() -> Result<String, String> {
    let context_root = match env::var("FLOW_CONTEXT_ROOT") {
        Ok(var) => PathBuf::from(&var),
        Err(_) => {
            let samples_dir = Path::new(env!("CARGO_MANIFEST_DIR")).parent()
                .ok_or("Could not get parent dir")?;
            samples_dir.join("flowr/src/bin/flowrcli/cli")
        }
    };
    assert!(context_root.exists(), "Context root directory '{}' does not exist", context_root.display());
    Ok(context_root.to_str().expect("Could not convert path to String").to_string())
}

fn compile_sample(sample_dir: &str, output_dir: &str) {
    let mut command = Command::new("flowc");
    // -d for debug symbols
    // -g to dump graphs
    // -v warn to show warnings
    // -c to skip running and only compile the flow
    // -O to optimize the WASM files generated
    // -C <dir> to set the context root dir
    // -o <output_dir> to generate output files in specified directory
    // <sample_dir> is the path to the directory of the sample flow to compile
    let context_root = get_context_root().expect("Could not get context root");
    let command_args = vec!["-d", "-g", "-c", "-O",
                            "-C", &context_root,
                            "-o", output_dir,
                            sample_dir];

    match command.args(&command_args).status() {
        Ok(stat) => {
            if !stat.success() {
                eprintln!("Error building sample, command line\n flowc {}",
                          command_args.join(" "));
                std::process::exit(1);
            }
        }
        Err(err) => {
            eprintln!("'{}' running command 'flowc {}'", err, command_args.join(" "));
            std::process::exit(1);
        }
    }
}

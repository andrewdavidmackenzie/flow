//! Build script to compile the flow flowsamples in the crate
use std::{fs, io};
use std::path::Path;
use std::process::Command;

fn main() -> io::Result<()> {
    let samples_root = env!("CARGO_MANIFEST_DIR");
    let root_dir = Path::new(samples_root).parent().expect("Could not get parent directory");
    let samples_out_dir = root_dir.join("target/flowsamples");

    println!("cargo:rerun-if-env-changed=FLOW_LIB_PATH");
    // Tell Cargo that if any file in the flowsamples directory changes it should rerun this build script
    println!("cargo:rerun-if-changed={}", samples_root);

    // find all sample sub-folders at have a "root.toml" flow definition file
    for entry in fs::read_dir(samples_root)? {
        let e = entry?;
        if e.file_type()?.is_dir() && e.path().join("root.toml").exists() {
            let sample_out_dir = &samples_out_dir.join(e.file_name());
            println!("Building sample '{}' to '{}'",
                     e.path().display(),
                     sample_out_dir.display());
            fs::create_dir_all(sample_out_dir).expect("Could not create output dir");
            compile_sample(&e.path().to_string_lossy(), &sample_out_dir.to_string_lossy());
        }
    }

    Ok(())
}

fn compile_sample(sample_dir: &str, output_dir: &str) {
    let mut command = Command::new("flowc");
    // -g for debug symbols, -z to dump graphs, -v warn to show warnings, -s to skip running and only compile the flow
    // -o output_dir to generate output files in specified directory
    let command_args = vec!["-g", "-z", "-v", "warn", "-s",
                            sample_dir, "-o", output_dir];

    if !command
        .args(&command_args).status().expect("Could not get status").success() {
        eprintln!("Error building sample, command line\n flowc {}",
                          command_args.join(" "));
        std::process::exit(1);
    }
}

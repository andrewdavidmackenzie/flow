// Build script to compile the flow samples in the crate

use std::{fs, io};
use std::path::Path;
use std::process::{Command, Stdio};

fn main() -> io::Result<()> {
    // find all sample sub-folders
    fs::read_dir(".")?
        .map(|res| res.map(|e| {
            if e.metadata().unwrap().is_dir() {
                compile_sample(&e.path());
            }
        }))
        .collect::<Result<Vec<_>, io::Error>>()?;

    println!("cargo:rerun-if-env-changed=FLOW_LIB_PATH");

    Ok(())
}

// @RUST_BACKTRACE=1 cargo run --quiet -p flowc -- -g -d $(@D) -i $< -- `cat $(@D)/test.arguments` 2> $(@D)/test.err > $@
// @diff $@ $(@D)/expected.output || (ret=$$?; cp $@ $(@D)/failed.output && rm -f $@ && rm -f $(@D)/test.file && exit $$ret)
// @if [ -s $(@D)/expected.file ]; then diff $(@D)/expected.file $(@D)/test.file; fi;
// @if [ -s $(@D)/test.err ]; then (printf " has error output in $(@D)/test.err\n"; exit -1); else printf " has no errors\n"; fi;
// @rm $@ #remove test.output after successful diff so that dependency will cause it to run again next time
// # leave test.err for inspection in case of failure
fn compile_sample(sample_dir: &Path) {
    // Tell Cargo that if the given file changes, to rerun this build script.
    println!("cargo:rerun-if-changed={}/context.toml", sample_dir.display());
    println!("cargo:rerun-if-changed={}/test.input", sample_dir.display());
    println!("cargo:rerun-if-changed={}/test.arguments", sample_dir.display());
    println!("cargo:rerun-if-changed={}/expected.file", sample_dir.display());
    println!("cargo:rerun-if-changed={}/expected.output", sample_dir.display());

    let mut command = Command::new("../target/debug/flowc");
    // -g for debug symbols, -d to dump compiler structs, -s to skip running, only compile the flow
    let command_args = vec!("-g", "-d", "-s", sample_dir.to_str().unwrap());

    command.args(command_args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn().unwrap();
}

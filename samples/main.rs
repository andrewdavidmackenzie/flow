// Run all the sample flows found in subfolders
// samples/%/test.output: samples/%/test.input samples/%/test.arguments
// @printf "\tSample '$(@D)'"
// @cat $< | RUST_BACKTRACE=1 cargo run --quiet -p flowr -- --native $(@D) `cat $(@D)/test.arguments` 2> $(@D)/test.err > $@
// @diff $@ $(@D)/expected.output || (ret=$$?; cp $@ $(@D)/failed.output && rm -f $@ && rm -f $(@D)/test.file && exit $$ret)
// @if [ -s $(@D)/expected.file ]; then diff $(@D)/expected.file $(@D)/test.file; fi;
// @if [ -s $(@D)/test.err ]; then (printf " has error output in $(@D)/test.err\n"; exit -1); else printf " has no errors\n"; fi;
// @rm $@ #remove test.output after successful diff so that dependency will cause it to run again next time
// # leave test.err for inspection in case of failure
fn main() {
    println!("running all the samples");
}
//
// fn test_args(test_dir: &PathBuf, test_name: &str) -> Vec<String> {
//     let test_args = format!("{}.args", test_name);
//     let mut args_file = test_dir.clone();
//     args_file.push(test_args);
//     let f = File::open(&args_file).unwrap();
//     let f = BufReader::new(f);
//
//     let mut args = Vec::new();
//     for line in f.lines() {
//         args.push(line.unwrap());
//     }
//     args
// }

// let mut command = Command::new("cargo");
// // -g for debug symbols, -d to dump compiler structs, -s to skip running, only compile the flow
// let mut command_args = vec!("run", "--quiet", "-p", "flowc", "--", "-g", "-d", "-s", sample.to_str().unwrap());
//

// // TODO read flow arguments from the file test.arguments
// let flow_args: Vec<&str> = vec!();
//
// for flow_arg in &flow_args {
// command_args.push(flow_arg);
// }

//
// // send it stdin from the "test.input" file
// write!(child.stdin.unwrap(), "{}/test.input", sample.display()).unwrap();
//
// // read stdout
// let mut output = String::new();
// if let Some(ref mut stdout) = child.stdout {
// for line in BufReader::new(stdout).lines() {
// output.push_str(&format!("{}\n", &line.unwrap()));
// }
// }
//
// // read stderr
// let mut err = String::new();
// if let Some(ref mut stderr) = child.stderr {
// for line in BufReader::new(stderr).lines() {
// err.push_str(&format!("{}\n", &line.unwrap()));
// }
// }

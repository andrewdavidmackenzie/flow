extern crate flow;
use flow::parser::parser;

// TODO a test that tests all samples???

#[test]
fn sample_hello_world_simple() {
    let sample_path = "samples/hello-world-simple/hello.context";
    match parser::load(sample_path, true) {
        parser::Result::ContextLoaded(_) => {},
        _ => assert!(false),
    }
}

#[test]
fn sample_hello_world() {
    let sample_path = "samples/hello-world/hello.context";
    match parser::load(sample_path, true) {
        parser::Result::ContextLoaded(_) => {},
        _ => assert!(false),
    }
}
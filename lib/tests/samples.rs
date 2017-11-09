extern crate flowlib;
use flowlib::parser::parser;

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
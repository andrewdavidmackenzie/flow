extern crate flowlib;
use flowlib::parser::parser;

use std::fs::File;

#[test]
fn sample_hello_world_simple() {
    let path = "../samples/hello-world-simple/hello.context";
    let mut file = File::open(path).unwrap();
    match parser::load(file) {
        parser::Result::ContextLoaded(_) => {},
        _ => assert!(false),
    }
}

#[test]
fn sample_hello_world() {
    let path = "../samples/hello-world/hello.context";
    let mut file = File::open(path).unwrap();
    match parser::load(file) {
        parser::Result::ContextLoaded(_) => {},
        _ => assert!(false),
    }
}
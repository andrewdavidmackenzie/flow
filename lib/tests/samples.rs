extern crate flowlib;
use flowlib::loader::loader;

use std::fs::File;

#[test]
fn sample_hello_world_simple() {
    let path = "../samples/hello-world-simple/hello.context";
    let file = File::open(path).unwrap();
    match loader::load(file) {
        loader::Result::Context(_) => {},
        _ => assert!(false),
    }
}

#[test]
fn sample_hello_world() {
    let path = "../samples/hello-world/hello.context";
    let file = File::open(path).unwrap();
    match loader::load(file) {
        loader::Result::Context(_) => {},
        _ => assert!(false),
    }
}
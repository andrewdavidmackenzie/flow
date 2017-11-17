use std::path::PathBuf;

extern crate flowlib;
use flowlib::loader::loader;
use flowlib::dumper::dump;

#[test]
fn sample_hello_world_simple() {
    let path = PathBuf::from("../samples/hello-world-simple/context.yaml");
    match loader::load(path) {
        loader::Result::Context(_) => {},
        _ => assert!(false),
    }
}

#[test]
fn sample_hello_world() {
    let path = PathBuf::from("../samples/hello-world/context.yaml");
    match loader::load(path) {
        loader::Result::Context(_) => {},
        _ => assert!(false),
    }
}

#[test]
fn dump_hello_world() {
    let path = PathBuf::from("../samples/hello-world/context.yaml");
    match loader::load(path) {
        loader::Result::Context(c) => dump(c),
        _ => assert!(false),
    }
}
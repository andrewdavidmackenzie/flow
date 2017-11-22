use std::path::PathBuf;

extern crate flowlib;
use flowlib::loader::loader;
use flowlib::dumper::dump;

#[test]
#[ignore]
fn sample_hello_world_simple() {
    let path = PathBuf::from("../samples/hello-world-simple/context.yaml");
    match loader::load(path) {
        Ok(_) => {},
        _ => assert!(false),
    }
}

#[test]
#[ignore]
fn sample_hello_world() {
    let path = PathBuf::from("../samples/hello-world/context.yaml");
    match loader::load(path) {
        Ok(_) => {},
        _ => assert!(false),
    }
}

#[test]
fn sample_hello_toml() {
    let path = PathBuf::from("../samples/hello-world-toml/context.toml");
    match loader::load(path) {
        Ok(_) => {},
        _ => assert!(false),
    }
}

#[test]
fn sample_hello_simple_toml() {
    let path = PathBuf::from("../samples/hello-world-simple-toml/context.toml");
    match loader::load(path) {
        Ok(_) => {},
        _ => assert!(false),
    }
}

#[test]
fn sample_hello_simple_message_toml() {
    let path = PathBuf::from("../samples/hello-world-simple-toml/message.toml");
    match loader::load(path) {
        Ok(_) => {},
        _ => assert!(false),
    }
}

#[test]
fn sample_hello_simple_terminal_toml() {
    let path = PathBuf::from("../samples/hello-world-simple-toml/terminal.toml");
    match loader::load(path) {
        Ok(_) => {},
        _ => assert!(false),
    }
}

#[test]
fn dump_hello_world_toml() {
    let path = PathBuf::from("../samples/hello-world-toml/context.toml");
    match loader::load(path) {
        Ok(f) => dump(f),
        _ => assert!(false),
    }
}

#[test]
fn dump_hello_world_flow1_toml() {
    let path = PathBuf::from("../samples/hello-world-toml/flow1.toml");
    match loader::load(path) {
        Ok(f) => dump(f),
        _ => assert!(false),
    }
}

#[test]
fn dump_hello_world_message_toml() {
    let path = PathBuf::from("../samples/hello-world-toml/message.toml");
    match loader::load(path) {
        Ok(f) => dump(f),
        _ => assert!(false),
    }
}

#[test]
fn dump_hello_world_terminal_toml() {
    let path = PathBuf::from("../samples/hello-world-toml/terminal.toml");
    match loader::load(path) {
        Ok(f) => dump(f),
        _ => assert!(false),
    }
}
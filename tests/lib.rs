extern crate flow;
use flow::parser::parser;

#[test]
fn can_load_sample() {
    let sample_path = "samples/hello-world-simple/hello.context";
    match parser::load(sample_path, true) {
        parser::Result::ContextLoaded(_) => {},
        _ => assert!(false),
    }
}
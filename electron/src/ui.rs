extern crate flowclib;
use flowclib::info;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

fn main() {
    println!("'flow' version: {}", VERSION);
    println!("'flowclib' version {}", info::version());
}
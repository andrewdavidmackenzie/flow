extern crate flowlib;
use flowlib::info;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

fn main() {
    println!("'flow' version: {}", VERSION);
    println!("'flowlib' version {}", info::version());
}
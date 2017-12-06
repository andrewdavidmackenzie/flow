extern crate flowrlib;
extern crate flowstdlib;

use values::values;
use functions::functions;

use flowrlib::execution::init;
use flowrlib::execution::looper;

mod functions;
mod values;

fn main() {
    println!("'{}' version {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
    println!("'flowrlib' version {}", flowrlib::info::version());
    println!("'flowstdlib' version {}", flowstdlib::info::version());

    // TODO some standard inputs that are passed to main as arguments
    // a library function to help parse them?

    init(&values);
    looper(&values, &functions);
}
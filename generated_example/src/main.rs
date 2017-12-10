extern crate flowrlib;
extern crate flowstdlib;

use flowrlib::execution::execute;

mod functions;
mod values;

use values::get_values;
use functions::get_functions;

fn main() {
    println!("'{}' version {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));

    // TODO some standard inputs that are passed to main as arguments
    // a library function to help parse them?

    execute(get_values(), get_functions());
}
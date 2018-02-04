use flowrlib::implementation::Implementation;

use std::fmt;
use std::fmt::Debug;

pub struct Stderr;

impl Implementation for Stderr {
    fn number_of_inputs(&self) -> usize {
        1
    }

    fn run(&self, mut inputs: Vec<Option<String>>) -> Option<String> {
        eprintln!("{}", inputs.remove(0).unwrap());
        None
    }

    fn name(&self) -> &'static str {
        "Stderr"
    }
}

impl Debug for Stderr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "stderr defined in file: '{}'", file!())
    }
}
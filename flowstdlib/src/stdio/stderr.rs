use flowrlib::function::Function;
use flowrlib::implementation::Implementation;

use std::fmt;
use std::fmt::Debug;

pub struct Stderr {
}

const DEFINITION: &'static str =
"name = 'Stderr'
[[input]]
name = 'stderr'
type = 'String'";

impl Implementation for Stderr {
    fn number_of_inputs(&self) -> usize {
        1
    }

    fn run(&self, function: &mut Function) {
        eprintln!("{}", function.inputs[0].as_ref().unwrap());
    }

    fn define(&self) -> &'static str {
        DEFINITION
    }
}

impl Debug for Stderr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "stderr defined in file: '{}'", file!())
    }
}
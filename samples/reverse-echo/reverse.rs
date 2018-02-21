use flowrlib::implementation::Implementation;

use std::fmt;
use std::fmt::Debug;

pub struct Reverse;

impl Implementation for Reverse {
    fn run(&self, inputs: Vec<Option<String>>) -> Option<String> {
        let input = inputs.remove(0).unwrap();

        // TODO reverse the string

        Some(input)
    }

    fn number_of_inputs(&self) -> usize {
        1
    }

    fn name(&self) -> &'static str {
        "Reverse"
    }
}

impl Debug for Stdin {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Implementation: stdin defined in file: '{}'", file!())
    }
}
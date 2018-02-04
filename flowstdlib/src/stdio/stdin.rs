use flowrlib::implementation::Implementation;

use std::fmt;
use std::fmt::Debug;

pub struct Stdin;

impl Implementation for Stdin {
    fn number_of_inputs(&self) -> usize {
        0
    }

    fn run(&self, _inputs: Vec<Option<String>>) -> Option<String> {
        use std::io::{self, Read};

        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer).unwrap();
        Some(buffer)
    }

    fn name(&self) -> &'static str {
        "Stdin"
    }
}

impl Debug for Stdin {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Implementation: stdin defined in file: '{}'", file!())
    }
}
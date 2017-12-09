use flowrlib::runnable::Runnable;
use flowrlib::implementation::Implementation;

use std::fmt;
use std::fmt::Debug;

pub struct Stdin;

const DEFINITION: &'static str ="
name = 'Stdin'
[[output]]
name = 'stdin'
type = 'String'";

impl Implementation for Stdin {
    fn number_of_inputs(&self) -> usize {
        0
    }

    fn run(&self, runnable: &mut Runnable) {
        use std::io::{self, Read};

        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer).unwrap();
        runnable.set_output(buffer);
    }

    fn define(&self) -> &'static str {
        DEFINITION
    }
}

impl Debug for Stdin {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Implementation: stdin defined in file: '{}'", file!())
    }
}
use flowrlib::runnable::Runnable;
use flowrlib::implementation::Implementation;

use std::fmt;
use std::fmt::Debug;

pub struct Stdout;

const DEFINITION: &'static str ="
name = 'Stdout'
[[input]]
name = 'stdout'
type = 'String'";

unsafe impl Sync for Stdout {}

impl Implementation for Stdout {
    fn number_of_inputs(&self) -> usize {
        1
    }

    fn run(&self, runnable: &mut Runnable) {
        println!("{:?}", runnable.read_input(0));
    }

    fn define(&self) -> &'static str {
        DEFINITION
    }
}

impl Debug for Stdout {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "stdout defined in file: '{}'", file!())
    }
}
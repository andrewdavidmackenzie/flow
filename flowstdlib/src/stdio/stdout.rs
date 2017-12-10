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

    fn run(&self, mut inputs: Vec<Option<String>>) -> Option<String> {
        println!("{:?}", inputs.remove(0).unwrap());
        None
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
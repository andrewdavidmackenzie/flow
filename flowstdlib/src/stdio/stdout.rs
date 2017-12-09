use flowrlib::function::Function;
use flowrlib::implementation::Implementation;

use std::fmt;
use std::fmt::Debug;

pub struct Stdout {
}

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

    fn run(&self, function: &mut Function) {
        println!("{:?}", function);

        // TODO gather my inputs if they are all there and call my implementation
        println!("{:?}", function.inputs[0].as_ref().unwrap());
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
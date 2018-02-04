use flowrlib::implementation::Implementation;

use std::fmt;
use std::fmt::Debug;

pub struct Stdout;

impl Implementation for Stdout {
    fn number_of_inputs(&self) -> usize {
        1
    }

    fn run(&self, mut inputs: Vec<Option<String>>) -> Option<String> {
        println!("{:?}", inputs.remove(0).unwrap());
        None
    }

    fn name(&self) -> &'static str {
        "Stdout"
    }
}

impl Debug for Stdout {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "stdout defined in file: '{}'", file!())
    }
}
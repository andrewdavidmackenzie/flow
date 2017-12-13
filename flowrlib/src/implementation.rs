use std::fmt::Debug;

pub trait Implementation: Debug {
    /*
        An implementation runs, receiving an array of inputs and possibly producing an output
    */
    fn run(&self, inputs: Vec<Option<String>>) -> Option<String>;

    fn number_of_inputs(&self) -> usize;

    fn name(&self) -> &str;

    fn define(&self) -> &'static str where Self: Sized;
}
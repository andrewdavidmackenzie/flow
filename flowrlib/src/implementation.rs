use function::Function;
use std::fmt::Debug;

pub trait Implementation: Debug {
    fn run(&self, &mut Function);
    fn number_of_inputs(&self) -> usize;
    fn define(&self) -> &'static str where Self: Sized;
}
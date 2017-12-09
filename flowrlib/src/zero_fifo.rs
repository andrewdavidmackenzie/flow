use implementation::Implementation;
use runnable::Runnable;

#[derive(Debug)]
pub struct Fifo;

impl Implementation for Fifo {
    fn run(&self, runnable: &mut Runnable) {
        let value = runnable.read_input(0);
        runnable.set_output(value);
        println!("run: value - copied input to output");
    }

    fn number_of_inputs(&self) -> usize {
        1
    }

    fn define(&self) -> &'static str where Self: Sized {
        "value"
    }
}
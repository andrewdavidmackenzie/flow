use implementation::Implementation;

#[derive(Debug)]
pub struct Fifo;

impl Implementation for Fifo {
    fn run(&self, mut inputs: Vec<Option<String>>) -> Option<String> {
        inputs.remove(0)
    }

    fn number_of_inputs(&self) -> usize {
        1
    }

    fn name(&self) -> &'static str {
        "Value"
    }
}
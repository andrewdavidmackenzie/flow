use implementation::Implementation;

pub struct Fifo;

impl Implementation for Fifo {
    fn run(&self, mut inputs: Vec<Option<String>>) -> Option<String> {
        inputs.remove(0)
    }
}
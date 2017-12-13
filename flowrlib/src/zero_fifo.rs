use implementation::Implementation;

#[derive(Debug)]
pub struct Fifo;

impl Implementation for Fifo {
    fn run(&self, mut inputs: Vec<Option<String>>) -> Option<String> {
        info!("run: Fifo - sole input provided to output");
        inputs.remove(0)
    }

    fn number_of_inputs(&self) -> usize {
        1
    }

    fn define(&self) -> &'static str where Self: Sized {
        "value"
    }

    fn name(&self) -> &'static str {
        "Value"
    }
}
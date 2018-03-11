pub trait Implementation {
    // An implementation runs, receiving an array of inputs and possibly producing an output
    fn run(&self, inputs: Vec<Option<String>>) -> Option<String>;
}
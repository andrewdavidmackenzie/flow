pub trait Runnable {
    fn name(&self) -> &str;
    fn number_of_inputs(&self) -> usize;
    fn id(&self) -> usize;
    fn init(&mut self) -> bool;
    fn write_input(&mut self, input_number: usize, new_value: Option<String>);
    fn inputs_satisfied(&self) -> bool;
    fn run(&mut self) -> Option<String>;
    fn output_destinations(&self) -> &Vec<(usize, usize)>;
}
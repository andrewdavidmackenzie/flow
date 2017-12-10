pub trait Runnable {
    fn init(&mut self) -> bool;
    fn write_input(&mut self, input_number: usize, new_value: String) -> bool;
    fn read_input(&mut self, input_number: usize) -> String;
    fn run(&mut self);
    fn set_output(&mut self, output_value: String);
}
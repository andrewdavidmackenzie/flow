pub trait Runnable {
    fn init(&mut self) -> bool;
    fn write_input(&mut self, input_number: usize, new_value: Option<String>) -> bool;
    fn run(&mut self)  -> Option<String>;
    fn get_affected(&self) -> Vec<(usize, usize)>;
}
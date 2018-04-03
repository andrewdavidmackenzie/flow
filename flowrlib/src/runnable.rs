use serde_json::Value as JsonValue;
use std::panic::RefUnwindSafe;
use std::panic::UnwindSafe;
use super::implementation::Implementation;

pub trait Runnable: RefUnwindSafe + UnwindSafe {
    fn name(&self) -> &str;
    fn number_of_inputs(&self) -> usize;
    fn id(&self) -> usize;
    fn init(&mut self) -> bool;
    fn write_input(&mut self, input_number: usize, new_value: JsonValue);
    fn input_full(&self, input_number: usize) -> bool;
    fn can_run(&self) -> bool; // This runnable has all the inputs necessary and can be run
    fn get_inputs(&mut self) -> Vec<Vec<JsonValue>>;
    fn output_destinations(&self) -> &Vec<(&'static str, usize, usize)>;
    fn implementation(&self) -> &Box<Implementation>;
}
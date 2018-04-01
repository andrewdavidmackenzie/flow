use serde_json::Value as JsonValue;
use std::panic::RefUnwindSafe;
use std::panic::UnwindSafe;
use super::implementation::Implementation;

pub trait Runnable : RefUnwindSafe + UnwindSafe {
    fn name(&self) -> &str;
    fn number_of_inputs(&self) -> usize;
    fn id(&self) -> usize;
    fn init(&mut self) -> bool;
    fn write_input(&mut self, input_number: usize, new_value: JsonValue);
    fn inputs_satisfied(&self) -> bool;
    fn get_inputs(&mut self) -> Vec<JsonValue>;
    fn output_destinations(&self) -> &Vec<(& 'static str, usize, usize)>;
    fn implementation(&self) -> &Box<Implementation>;
}
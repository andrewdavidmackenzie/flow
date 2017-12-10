use runnable::Runnable;
use implementation::Implementation;
use zero_fifo::Fifo;
use std::mem::replace;

const ONLY_INPUT: usize = 0;

#[derive(Debug)]
pub struct Value {
    id: u32,

    initial_value: Option<&'static str>,
    implementation: &'static Implementation,

    num_inputs: usize,
    num_inputs_pending: usize,
    inputs: Vec<Option<String>>,

    output_routes: Vec<(usize, usize)>
}

impl Value {
    pub fn new(id: u32, initial_value: Option<&'static str>,
               output_routes: Vec<(usize, usize)>) -> Value {
        let number_of_inputs = 1;

        Value {
            id,
            initial_value,
            implementation: &Fifo,
            num_inputs: number_of_inputs,
            num_inputs_pending: number_of_inputs,
            inputs: vec![None; number_of_inputs],
            output_routes
        }
    }
}

impl Runnable for Value {
    /*
        If an initial value is defined then write it to the current value.
        Return true if ready to run as all inputs (single in this case) are satisfied.
    */
    fn init(&mut self) -> bool {
        if let Some(new_value) = self.initial_value {
            return self.write_input(ONLY_INPUT, Some(new_value.to_string()));
        }
        false
    }

    /*
        Update the value stored - this should only be called when the input is available and the
        value has already been consumed by all the listeners and hence it can be overwritten.
    */
    fn write_input(&mut self, input_number: usize, input_value: Option<String>) -> bool {
        self.num_inputs_pending -= 1;
        self.inputs[input_number] = input_value;
        self.num_inputs_pending == 0 // all inputs satisfied
    }

    /*
        A Runnable is run by running the actual implementation and passing in the inputs
    */
    fn run(&mut self) -> Option<String> {
        // Consume the inputs
        let inputs = replace(&mut self.inputs, vec![None; self.num_inputs]);
        self.implementation.run(inputs)
    }

    fn get_affected(&self) -> Vec<(usize, usize)> {
        self.output_routes.clone()
    }
}
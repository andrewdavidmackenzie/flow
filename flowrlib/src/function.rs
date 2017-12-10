use runnable::Runnable;
use implementation::Implementation;
use std::mem::replace;

#[derive(Debug)]
pub struct Function {
    id: u32,

    initial_value: Option<&'static str>,
    implementation: &'static Implementation,

    num_inputs: usize,
    num_inputs_pending: usize,
    inputs: Vec<Option<String>>,

    output_routes: Vec<(usize, usize)>
}

// TODO these methods will need to be made thread safe

// TODO Make these doc comments and produce some documentation?

impl Function {
    pub fn new(id: u32, implementation: &'static Implementation, output_routes: Vec<(usize, usize)>)
               -> Function {
        let number_of_inputs = implementation.number_of_inputs();
        Function {
            id,
            initial_value: None,
            implementation,
            num_inputs: number_of_inputs,
            num_inputs_pending: number_of_inputs,
            inputs: vec![None; number_of_inputs],
            output_routes
        }
    }
}

impl Runnable for Function {
    fn init(&mut self) -> bool { false }

    /*
        provide a given input
    */
    fn write_input(&mut self, input_number: usize, input_value: Option<String>) -> bool {
        self.num_inputs_pending -= 1;
        self.inputs[input_number] = input_value;
        self.num_inputs_pending == 0 // all inputs satisfied
    }

    fn run(&mut self) -> Option<String> {
        // Consume the inputs
        let inputs = replace(&mut self.inputs, vec![None; self.num_inputs]);
        self.implementation.run(inputs)
    }

    fn get_affected(&self) -> Vec<(usize, usize)> {
        self.output_routes.clone()
    }
}
use runnable::Runnable;
use implementation::Implementation;
use std::mem::replace;

pub struct Function {
    name: String,
    number_of_inputs: usize,
    id: usize,
    implementation: Box<Implementation>,

    num_inputs_pending: usize,
    inputs: Vec<Option<String>>,

    output_routes: Vec<(usize, usize)>,
}

impl Function {
    pub fn new(name: String, number_of_inputs: usize, id: usize, implementation: Box<Implementation>,
               output_routes: Vec<(usize, usize)>)
               -> Function {
        Function {
            name,
            number_of_inputs,
            id,
            implementation,
            num_inputs_pending: number_of_inputs,
            inputs: vec![None; number_of_inputs],
            output_routes,
        }
    }
}

impl Runnable for Function {
    fn name(&self) -> &str { &self.name }

    fn number_of_inputs(&self) -> usize { self.number_of_inputs }

    fn id(&self) -> usize { self.id }

    // If a function has zero inputs it is considered ready to run at init
    fn init(&mut self) -> bool {
        self.number_of_inputs == 0
    }

    fn write_input(&mut self, input_number: usize, input_value: Option<String>) {
        self.num_inputs_pending -= 1;
        self.inputs[input_number] = input_value;
    }

    // responds true if all inputs have been satisfied - false otherwise
    fn inputs_satisfied(&self) -> bool {
        self.num_inputs_pending == 0
    }

    // Consume the inputs, reset the number of pending inputs and run the implementation
    fn run(&mut self) -> Option<String> {
        let inputs = replace(&mut self.inputs, vec![None; self.number_of_inputs]);
        self.num_inputs_pending = self.number_of_inputs;
        self.implementation.run(inputs)
    }

    fn output_destinations(&self) -> &Vec<(usize, usize)> {
        &self.output_routes
    }
}

use runnable::Runnable;
use implementation::Implementation;
use std::mem::replace;

pub struct Function {
    id: usize,
    implementation: Box<Implementation>,

    num_inputs: usize,
    num_inputs_pending: usize,
    inputs: Vec<Option<String>>,

    output_routes: Vec<(usize, usize)>,
}

impl Function {
    pub fn new(id: usize, implementation: Box<Implementation>,
               output_routes: Vec<(usize, usize)>)
               -> Function {
        let number_of_inputs = implementation.number_of_inputs();
        Function {
            id,
            implementation,
            num_inputs: number_of_inputs,
            num_inputs_pending: number_of_inputs,
            inputs: vec![None; number_of_inputs],
            output_routes,
        }
    }
}

impl Runnable for Function {
    fn id(&self) -> usize { self.id }

    // If a function has zero inputs it is considered ready to run at init
    fn init(&mut self) -> bool {
        self.num_inputs == 0
    }

    fn write_input(&mut self, input_number: usize, input_value: Option<String>) {
        self.num_inputs_pending -= 1;
        self.inputs[input_number] = input_value;
    }

    // responds true if all inputs have been satisfied - false otherwise
    fn inputs_satisfied(&self) -> bool {
        self.num_inputs_pending == 0
    }

    /*
        Consume the inputs, reset the number of pending inputs and run the implementation
    */
    fn run(&mut self) -> Option<String> {
        let inputs = replace(&mut self.inputs, vec![None; self.num_inputs]);
        self.num_inputs_pending = self.num_inputs;
        info!("Running implementation: '{}'", &self.implementation.name());
        self.implementation.run(inputs)
    }

    fn output_destinations(&self) -> &Vec<(usize, usize)> {
        &self.output_routes
    }
}

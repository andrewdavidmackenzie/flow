use runnable::Runnable;
use implementation::Implementation;
use zero_fifo::Fifo;
use std::mem::replace;

const ONLY_INPUT: usize = 0;

pub struct Value {
    id: usize,
    initial_value: Option<String>,
    implementation: Box<Implementation>,
    input: Option<String>,
    output_routes: Vec<(usize, usize)>
}

impl Value {
    pub fn new(id: usize, initial_value: Option<String>, output_routes: Vec<(usize, usize)>) -> Value {
        Value {
            id,
            initial_value,
            implementation: Box::new(Fifo),
            input: None,
            output_routes
        }
    }
}

impl Runnable for Value {
    fn id(&self) -> usize { self.id }

    /*
        If an initial value is defined then write it to the current value.
        Return true if ready to run as all inputs (single in this case) are satisfied.
    */
    fn init(&mut self) -> bool {
        let value = self.initial_value.clone();
        if value.is_some() {
            info!("Value initialized by writing '{:?}' to input", &value);
            self.write_input(ONLY_INPUT, value);
        }
        self.inputs_satisfied()
    }

    /*
        Update the value stored - this should only be called when the value has already been
        consumed by all the listeners and hence it can be overwritten.
    */
    fn write_input(&mut self, _input_number: usize, input_value: Option<String>) {
        self.input = input_value;
    }

    /*
        Responds true if all inputs have been satisfied - false otherwise
    */
    fn inputs_satisfied(&self) -> bool {
        self.input.is_some()
    }

    /*
        Consume the inputs and pass them to the actual implementation
    */
    fn run(&mut self) -> Option<String> {
        let input = replace(&mut self.input, None);
        info!("Running implementation: '{}'", &self.implementation.name());
        self.implementation.run(vec!(input))
    }

    fn output_destinations(&self) -> &Vec<(usize, usize)> {
        &self.output_routes
    }
}
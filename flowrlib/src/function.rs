use runnable::Runnable;
use implementation::Implementation;
use std::mem::replace;

#[derive(Debug)]
pub struct Function {
    initial_value: Option<&'static str>,
    implementation: &'static Implementation,

    num_inputs: usize,
    num_inputs_pending: usize,
    inputs: Vec<Option<String>>,

    num_listeners: usize,         // How many listeners are listening on this value
    pending_reads: usize,      // How many "reads" of the value are needed before it's empty
    output: Option<String>
}

// TODO these methods will need to be made thread safe

// TODO Make these doc comments and produce some documentation?

impl Function {
    pub fn new(implementation: &'static Implementation, num_listeners: usize) -> Function {
        let number_of_inputs = implementation.number_of_inputs();
        Function {
            initial_value: None,
            implementation,
            num_inputs: number_of_inputs,
            num_inputs_pending: number_of_inputs,
            inputs: vec![None; number_of_inputs],
            num_listeners,
            pending_reads: num_listeners,
            output: None
        }
    }

    /*
        This method is called when the function has just been ran
    */
    pub fn ran(&mut self) {
        self.pending_reads = self.num_listeners;
        self.num_inputs_pending = self.num_inputs;
    }

    /*
        This method should only be called when the output is known to be not None
    */
    pub fn read(&mut self) -> String {
        self.pending_reads -= 1;
        let value = self.output.clone().unwrap();
        if self.pending_reads == 0 {
            self.output = None;
        }
        value
    }
}

impl Runnable for Function {
    /*
        provide a given input
    */
    fn write_input(&mut self, input_number: usize, input_value: String) -> bool {
        self.num_inputs_pending -=1;
        self.inputs[input_number] = Some(input_value);
        self.num_inputs_pending == 0 // all inputs satisfied
    }

    fn read_input(&mut self, input_number: usize) -> String {
        replace(&mut self.inputs[input_number], None).unwrap()
    }

    fn run(&mut self) {
        self.implementation.run(self);
    }

    fn set_output(&mut self, output_value: String) {
        self.output = Some(output_value);
    }
}
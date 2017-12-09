use implementation::Implementation;

#[derive(Debug)]
pub struct Function {
    pub implementation: &'static Implementation,
    pub num_inputs: usize,
    pub num_inputs_pending: usize,
    pub inputs: Vec<Option<String>>,
    pub num_listeners: usize,         // How many listeners are listening on this value
    pub pending_reads: usize,      // How many "reads" of the value are needed before it's empty
    pub output: Option<String>
}

// TODO these methods will need to be made thread safe

// TODO Make these doc comments and produce some documentation?

impl Function {
    pub fn new(implementation: &'static Implementation, num_listeners: usize) -> Function {
        let number_of_inputs = implementation.number_of_inputs();
        Function {
            implementation,
            num_inputs: number_of_inputs,
            num_inputs_pending: number_of_inputs,
            inputs: Vec::<Option<String>>::with_capacity(number_of_inputs),
            num_listeners,
            pending_reads: num_listeners,
            output: None
        }
    }

    /*
        provide a given input
    */
    pub fn write(&mut self, input_number: usize, value: String) {
        self.num_inputs_pending -=1;
        self.inputs[input_number] = Some(value);
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
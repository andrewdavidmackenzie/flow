use runnable::Runnable;
use implementation::Implementation;
use zero_fifo::Fifo;
use std::mem::replace;

const ONLY_INPUT: usize = 0;

#[derive(Debug)]
pub struct Value {
    pub initial_value: Option<&'static str>,
    pub implementation: &'static Implementation,
    pub value: Option<String>,
    pub output: Option<String>,
    pub num_listeners: usize,       // How many listeners are listening on this value
    pub pending_reads: usize        // How many "reads" of the value are needed before it's empty
}

impl Value {
    pub fn new(initial_value: Option<&'static str>, num_listeners: usize) -> Value {
        Value {
            initial_value,
            implementation: &Fifo,
            value: None,
            output: None,
            num_listeners,
            pending_reads: 0
        }
    }

    /*
        If an initial value is defined then write it to the current value.
        Return true if ready to run as all inputs (single in this case) are satisfied.
    */
    pub fn init(&mut self) -> bool {
        if let Some(new_value) = self.initial_value {
            return self.write_input(ONLY_INPUT, new_value.to_string());
        }
        false
    }

    /*
        This method should only be called when the value is known to be not None
    */
    pub fn read(&mut self) -> String {
        self.pending_reads -= 1;
        let value = self.value.clone().unwrap();
        if self.pending_reads == 0 {
            self.value = None;
        }
        value
    }
}

impl Runnable for Value {
    /*
        Update the value stored - this should only be called when the input is available and the
        value has already been consumed by all the listeners and hence it can be overwritten.
    */
    fn write_input(&mut self, _input_number: usize, input_value: String) -> bool {
        self.value = Some(input_value);
        true // inputs are all satisfied
    }

    fn read_input(&mut self, _input_number: usize) -> String {
        replace(&mut self.value, None).unwrap()
    }

    fn run(&mut self) {
        self.implementation.run(self);
    }

    fn set_output(&mut self, output_value: String) {
        self.pending_reads = self.num_listeners;
        self.output = Some(output_value);
    }
}
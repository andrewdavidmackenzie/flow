#[derive(Debug)]
pub struct Value {
    pub initial_value: Option<&'static str>,
    pub value: Option<String>,
    pub num_listeners: usize,     // How many listeners are listening on this value
    pub pending_reads: usize      // How many "reads" of the value are needed before it's empty
}

// TODO Make these doc comments and produce some documentation?

impl Value {
    pub fn new(initial_value: Option<&'static str>, num_listeners: usize) -> Value {
        Value {
            initial_value,
            value: None,
            num_listeners,
            pending_reads: 0
        }
    }

    /*
        If an initial value is defined then write it to the current value
    */
    pub fn init(&mut self) {
        if let Some(new_value) = self.initial_value {
            self.write(new_value);
        }
    }

    /*
        Update the value stored - this should only be called when the input is available and the
        value has already been consumed by all the listeners and hence it can be overwritten.
    */
    pub fn write(&mut self, new_value: &str) {
        self.pending_reads = self.num_listeners;
        self.value = Some(new_value.to_string());
        println!("value updated to: {:?}", &self.value);

        // TODO he we need to either provide the values to the listeners that can accept it
        // or mark them somehow so that the scheduler knows to come and get the value.
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
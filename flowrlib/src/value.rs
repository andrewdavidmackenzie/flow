use serde_json::Value as JsonValue;
use runnable::Runnable;
use implementation::Implementation;
use zero_fifo::Fifo;

const ONLY_INPUT: usize = 0;

pub struct Value {
    name: String,
    id: usize,
    initial_value: Option<JsonValue>,
    implementation: Box<Implementation>,
    input: JsonValue,
    output_routes: Vec<(usize, usize)>
}

impl Value {
    pub fn new(name: String, id: usize, initial_value: Option<JsonValue>, output_routes: Vec<(usize, usize)>) -> Value {
        Value {
            name,
            id,
            initial_value,
            implementation: Box::new(Fifo),
            input: JsonValue::Null,
            output_routes
        }
    }
}

impl Runnable for Value {
    fn name(&self) -> &str {
        &self.name
    }

    fn number_of_inputs(&self) -> usize { 1 }

    fn id(&self) -> usize { self.id }

    /*
        If an initial value is defined then write it to the current value.
        Return true if ready to run as all inputs (single in this case) are satisfied.
    */
    fn init(&mut self) -> bool {
        let value = self.initial_value.clone();
        if let Some(v) = value {
            debug!("Value initialized by writing '{:?}' to input", &v);
            self.write_input(ONLY_INPUT, v);
        }
        self.inputs_satisfied()
    }

    /*
        Update the value stored - this should only be called when the value has already been
        consumed by all the listeners and hence it can be overwritten.
    */
    fn write_input(&mut self, _input_number: usize, input_value: JsonValue) {
        self.input = input_value;
    }

    /*
        Responds true if all inputs have been satisfied - false otherwise
    */
    fn inputs_satisfied(&self) -> bool {
        !self.input.is_null()
    }

    /*
        Consume the inputs and pass them to the actual implementation
    */
    fn run(&mut self) -> JsonValue {
        let input = self.input.take();
        self.implementation.run(vec!(input))
    }

    fn output_destinations(&self) -> &Vec<(usize, usize)> {
        &self.output_routes
    }
}
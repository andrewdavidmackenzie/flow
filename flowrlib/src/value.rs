use runnable::Runnable;
use implementation::Implementation;
use zero_fifo::Fifo;
use std::mem::replace;

const ONLY_INPUT: usize = 0;

#[derive(Debug)]
pub struct Value {
    initial_value: Option<String>,
    implementation: Box<Implementation>,

    num_inputs: usize,
    num_inputs_pending: usize,
    inputs: Vec<Option<String>>,

    output_routes: Vec<(usize, usize)>
}

impl Value {
    pub fn new(initial_value: Option<String>,
               output_routes: Vec<(usize, usize)>) -> Value {
        let number_of_inputs = 1;

        Value {
            initial_value,
            implementation: Box::new(Fifo),
            num_inputs: number_of_inputs,
            num_inputs_pending: number_of_inputs,
            inputs: vec![None; number_of_inputs],
            output_routes
        }
    }
}

#[test]
fn value_to_code() {
    let value = Value::new(Some("Hello-World".to_string()),
                           vec!((1,0)));
    let code = value.to_code();
    assert_eq!(code, "Value::new(Some(\"Hello-World\".to_string()), vec!((1,0),))")
}

impl Runnable for Value {
    /*
        If an initial value is defined then write it to the current value.
        Return true if ready to run as all inputs (single in this case) are satisfied.
    */
    fn init(&mut self) -> bool {
        let value = self.initial_value.clone();
        if value.is_some() {
            info!("Value initialized by writing '{:?}' to input", &value);
            return self.write_input(ONLY_INPUT, value);
        }
        false // have no value set
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

    // example   "Value::new(Some(\"Hello-World\".to_string()), vec!((1,0)))"
    fn to_code(&self) -> String {
        let mut code = "Value::new(".to_string();
        let value = self.initial_value.clone();
        if value.is_none() {
            code.push_str("None");
        } else {
            code.push_str(&format!("Some(\"{}\".to_string()),", value.unwrap()));
        }
        // Add the vector of tuples of runnables and their inputs it's connected to
        code.push_str(" vec!(");
        for ref route in &self.output_routes {
            code.push_str(&format!("({},{}),", route.0, route.1));
        }
        code.push_str(")");

        code.push_str(")");
        code
    }
}
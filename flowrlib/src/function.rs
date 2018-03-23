use serde_json::Value as JsonValue;
use runnable::Runnable;
use implementation::Implementation;
use std::mem::replace;

pub struct Function {
    name: String,
    number_of_inputs: usize,
    id: usize,
    implementation: Box<Implementation>,

    num_inputs_pending: usize,
    inputs: Vec<JsonValue>,

    output_routes: Vec<(& 'static str, usize, usize)>,
}

impl Function {
    pub fn new(name: String, number_of_inputs: usize, id: usize, implementation: Box<Implementation>,
               output_routes: Vec<(& 'static str, usize, usize)>)
               -> Function {
        Function {
            name,
            number_of_inputs,
            id,
            implementation,
            num_inputs_pending: number_of_inputs,
            inputs: vec![JsonValue::Null; number_of_inputs],
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

    fn write_input(&mut self, input_number: usize, input_value: JsonValue) {
        self.num_inputs_pending -= 1;
        self.inputs[input_number] = input_value;
    }

    // responds true if all inputs have been satisfied - false otherwise
    fn inputs_satisfied(&self) -> bool {
        self.num_inputs_pending == 0
    }

    // Consume the inputs, reset the number of pending inputs and run the implementation
    fn run(&mut self) -> JsonValue {
        let inputs = replace(&mut self.inputs, vec![JsonValue::Null; self.number_of_inputs]);
        self.num_inputs_pending = self.number_of_inputs;
        self.implementation.run(inputs)
    }

    fn output_destinations(&self) -> &Vec<(& 'static str, usize, usize)> {
        &self.output_routes
    }
}


#[cfg(test)]
mod test {
    use super::Function;
    use super::super::implementation::Implementation;
    use serde_json::value::Value as JsonValue;

    struct TestFunction;

    impl Implementation for TestFunction {
        fn run(&self, mut inputs: Vec<JsonValue>) -> JsonValue {
            inputs.remove(0)
        }
    }

    #[test]
    fn destructure_output_base_route() {
        let function = Function {
            name: "test_function".to_string(),
            number_of_inputs: 0,
            id: 0,
            implementation: Box::new(TestFunction),
            num_inputs_pending: 0,
            inputs: vec!(),
            output_routes: vec!(("", 1, 0)),
        };

        let output = function.implementation.run(vec!(json!("simple")));

        assert_eq!(output.pointer("").unwrap(), "simple");
    }

    #[test]
    fn destructure_json_value() {
        let json: JsonValue = json!({ "sub_route": "sub_output" });

        let function = Function {
            name: "test_function".to_string(),
            number_of_inputs: 0,
            id: 0,
            implementation: Box::new(TestFunction),
            num_inputs_pending: 0,
            inputs: vec!(),
            output_routes: vec!(("", 1, 0)),
        };

        let output = function.implementation.run(vec!(json.clone()));
        assert_eq!(output.pointer("/sub_route").unwrap(), "sub_output");
    }
}
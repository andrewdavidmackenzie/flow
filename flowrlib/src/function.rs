use serde_json::Value as JsonValue;
use runnable::Runnable;
use implementation::Implementation;
use std::panic::RefUnwindSafe;
use std::panic::UnwindSafe;
use input::Input;

pub struct Function {
    name: String,
    number_of_inputs: usize,
    id: usize,
    implementation: Box<Implementation>,
    inputs: Vec<Input>,
    output_routes: Vec<(&'static str, usize, usize)>,
}

impl Function {
    pub fn new(name: &str,
               number_of_inputs: usize,
               _reading_consumes: bool,
               input_depths: Vec<usize>,
               id: usize,
               implementation: Box<Implementation>,
               _initial_value: Option<JsonValue>,
               output_routes: Vec<(&'static str, usize, usize)>)
               -> Function {
        let mut function = Function {
            name: name.to_string(),
            number_of_inputs,
            id,
            implementation,
            inputs: Vec::with_capacity(number_of_inputs),
            output_routes,
        };

        for input_depth in input_depths {
            function.inputs.push(Input::new(input_depth));
        }

        function
    }
}

impl RefUnwindSafe for Function {}

impl UnwindSafe for Function {}

impl Runnable for Function {
    fn name(&self) -> &str { &self.name }

    fn number_of_inputs(&self) -> usize { self.number_of_inputs }

    fn id(&self) -> usize { self.id }

    // If a function has zero inputs can be ready to run without receiving any input
    fn init(&mut self) -> bool {
        self.can_run()
    }

    fn write_input(&mut self, input_number: usize, input_value: JsonValue) {
        if self.inputs[input_number].full() {
            error!("\t\tRunnable #{} '{}' Input overflow on input number {}", self.id(), self.name(), input_number);
        } else {
            self.inputs[input_number].push(input_value);
        }
    }

    fn input_full(&self, input_number: usize) -> bool {
        self.inputs[input_number].full()
    }

    // responds true if all inputs have been satisfied and this runnable can be run - false otherwise
    fn can_run(&self) -> bool {
        for input in &self.inputs {
            if !input.full() {
                return false;
            }
        }

        return true;
    }

    fn get_inputs(&mut self) -> Vec<Vec<JsonValue>> {
        let mut inputs: Vec<Vec<JsonValue>> = Vec::new();
        for input in &mut self.inputs {
            inputs.push(input.get());
        }
        inputs
    }

    fn output_destinations(&self) -> &Vec<(&'static str, usize, usize)> { &self.output_routes }

    fn implementation(&self) -> &Box<Implementation> { &self.implementation }
}


#[cfg(test)]
mod test {
    use serde_json::value::Value as JsonValue;

    #[test]
    fn destructure_output_base_route() {
        let json = json!("simple");
        assert_eq!(json.pointer("").unwrap(), "simple");
    }

    #[test]
    fn destructure_json_value() {
        let json: JsonValue = json!({ "sub_route": "sub_output" });
        assert_eq!(json.pointer("/sub_route").unwrap(), "sub_output");
    }

    #[test]
    fn access_array_elements() {
        let args: Vec<&str> = vec!("arg0", "arg1", "arg2");
        let json = json!(args);
        assert_eq!(json.pointer("/0").unwrap(), "arg0");
        assert_eq!(json.pointer("/1").unwrap(), "arg1");
    }
}
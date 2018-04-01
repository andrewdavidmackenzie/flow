use serde_json::Value as JsonValue;
use runnable::Runnable;
use implementation::Implementation;
use std::mem::replace;
use std::panic::RefUnwindSafe;
use std::panic::UnwindSafe;

pub struct Function {
    name: String,
    number_of_inputs: usize,
    id: usize,
    implementation: Box<Implementation>,

    num_inputs_pending: usize,
    inputs: Vec<JsonValue>,

    output_routes: Vec<(&'static str, usize, usize)>,
}

impl Function {
    pub fn new(name: String,
               number_of_inputs: usize,
               id: usize,
               implementation: Box<Implementation>,
               _initial_value: Option<JsonValue>,
               output_routes: Vec<(&'static str, usize, usize)>)
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

impl RefUnwindSafe for Function {}
impl UnwindSafe for Function {}

impl Runnable for Function {
    fn name(&self) -> &str { &self.name }

    fn number_of_inputs(&self) -> usize { self.number_of_inputs }

    fn id(&self) -> usize { self.id }

    // If a function has zero inputs it is considered ready to run any time it's not blocked on output
    fn init(&mut self) -> bool {
        self.inputs_satisfied()
    }

    fn write_input(&mut self, input_number: usize, input_value: JsonValue) {
        if self.inputs[input_number] != JsonValue::Null {
            error!("Overwriting input that has not been consumed");
        }
        self.inputs[input_number] = input_value;
        self.num_inputs_pending -= 1;
    }

    // responds true if all inputs have been satisfied - false otherwise
    fn inputs_satisfied(&self) -> bool {
        self.num_inputs_pending == 0
    }

    fn get_inputs(&mut self) -> Vec<JsonValue> {
        let inputs = replace(&mut self.inputs, vec![JsonValue::Null; self.number_of_inputs]);
        self.num_inputs_pending = self.number_of_inputs;
        inputs
    }

    fn output_destinations(&self) -> &Vec<(&'static str, usize, usize)> { &self.output_routes }

    fn implementation(&self) -> &Box<Implementation> { &self.implementation }
}


#[cfg(test)]
mod test {
    use super::super::implementation::Implementation;
    use serde_json::value::Value as JsonValue;
    use super::super::runlist::RunList;
    use super::super::runnable::Runnable;

    struct TestFunction;

    impl Implementation for TestFunction {
        fn run(&self, runnable: &Runnable, mut inputs: Vec<JsonValue>, run_list: &mut RunList) {
            run_list.send_output(runnable, inputs.remove(0));
        }
    }

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
}
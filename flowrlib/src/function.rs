use implementation::Implementation;
use implementation::RunAgain;
use input::Input;
use serde_json::Value;
use std::sync::Arc;
#[cfg(feature = "debugger")]
use std::fmt;

#[derive(Deserialize, Serialize)]
pub struct Function {
    #[cfg(feature = "debugger")]
    #[serde(default, skip_serializing_if = "String::is_empty")]
    name: String,

    #[cfg(feature = "debugger")]
    #[serde(default, skip_serializing_if = "String::is_empty")]
    route: String,

    id: usize,

    implementation_source: String,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    inputs: Vec<Input>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    output_routes: Vec<(String, usize, usize)>,

    #[serde(skip)]
    #[serde(default = "Function::default_implementation")]
    implementation: Arc<Implementation>,

    #[serde(default, skip_serializing_if = "Self::is_pure")]
    impure: bool,
}

struct ImplementationNotFound;

impl Implementation for ImplementationNotFound {
    fn run(&self, _inputs: Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        error!("Implementation not found");
        (None, false)
    }
}

#[cfg(feature = "debugger")]
impl fmt::Display for Function {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Function #{} '{}'\n", self.id, self.name)?;
        for (number, input) in self.inputs.iter().enumerate() {
            if input.is_empty() {
                write!(f, "\tInput :{} empty\n", number)?;
            } else {
                write!(f, "\tInput :{} {}\n", number, input)?;
            }
        }
        for output_route in &self.output_routes {
            if output_route.0.is_empty() {
                write!(f, "\tOutput route '/{}' -> {}:{}\n", output_route.0, output_route.1, output_route.2)?;
            } else {
                write!(f, "\tOutput route '{}' -> {}:{}\n", output_route.0, output_route.1, output_route.2)?;
            }
        }
        write!(f, "")
    }
}

impl Function {
    pub fn new(name: String,
               route: String,
               implementation_source: String,
               impure: bool,
               inputs: Vec<Input>,
               id: usize,
               output_routes: &Vec<(String, usize, usize)>) -> Function {
        Function {
            name,
            route,
            id,
            implementation_source,
            implementation: Function::default_implementation(),
            output_routes: (*output_routes).clone(),
            inputs,
            impure,
        }
    }

    /*
        Reset to initial state
    */
    pub fn reset(&mut self) {
        for input in &mut self.inputs {
            input.reset();
        }
    }

    pub fn default_implementation() -> Arc<Implementation> {
        Arc::new(super::function::ImplementationNotFound {})
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn id(&self) -> usize {
        self.id
    }

    /*
        Initialize all inputs - as they may have initializers
    */
    pub fn init_inputs(&mut self, first_time: bool) -> Vec<usize> {
        let mut refilled = vec!();
        for (io_number, mut input) in &mut self.inputs.iter_mut().enumerate() {
            if input.init(first_time) {
                refilled.push(io_number);
            }
        }
        refilled
    }

    pub fn implementation_source(&self) -> &str {
        &self.implementation_source
    }

    pub fn is_impure(&self) -> bool { self.impure }

    pub fn is_pure(field: &bool) -> bool { !*field }

    /*
        The value being sent maybe an Array of values, in which case if the destination input does
        not accept Array, then iterate over the contents of the array and send each one to the
        input individually
    */
    pub fn write_input(&mut self, input_number: usize, input_value: Value) {
        let input = &mut self.inputs[input_number];
        if input_value.is_array() && !input.is_array {
            debug!("Serializing Array value to non-Array input");
            for value in input_value.as_array().unwrap().iter() {
                input.push(value.clone());
            }
        } else {
            input.push(input_value);
        }
    }

    pub fn output_destinations(&self) -> &Vec<(String, usize, usize)> {
        &self.output_routes
    }

    pub fn get_implementation(&self) -> Arc<Implementation> {
        self.implementation.clone()
    }

    pub fn set_implementation(&mut self, implementation: Arc<Implementation>) {
        self.implementation = implementation;
    }

    pub fn input_full(&self, input_number: usize) -> bool {
        self.inputs[input_number].full()
    }

    // responds true if all inputs have been satisfied and this function can be run - false otherwise
    pub fn inputs_full(&self) -> bool {
        for input in &self.inputs {
            if !input.full() {
                return false;
            }
        }

        return true;
    }

    #[cfg(feature = "debugger")]
    pub fn inputs(&self) -> &Vec<Input> {
        &self.inputs
    }

    /*
        Read the values from the inputs and return them for use in executing the function
    */
    pub fn take_input_set(&mut self) -> Vec<Vec<Value>> {
        let mut input_set: Vec<Vec<Value>> = Vec::new();
        for input in &mut self.inputs {
            input_set.push(input.take());
        }
        input_set
    }
}

#[cfg(test)]
mod test {
    use serde_json::value::Value;
    use super::Function;
    use input::Input;

    #[test]
    fn destructure_output_base_route() {
        let json = json!("simple");
        assert_eq!(json.pointer("").unwrap(), "simple");
    }

    #[test]
    fn destructure_json_value() {
        let json: Value = json!({ "sub_route": "sub_output" });
        assert_eq!(json.pointer("/sub_route").unwrap(), "sub_output");
    }

    #[test]
    fn access_array_elements() {
        let args: Vec<&str> = vec!("arg0", "arg1", "arg2");
        let json = json!(args);
        assert_eq!(json.pointer("/0").unwrap(), "arg0");
        assert_eq!(json.pointer("/1").unwrap(), "arg1");
    }

    #[test]
    fn can_send_input_if_empty() {
        let mut function = Function::new("test".to_string(),
                                         "/context/test".to_string(),
                                         "/test".to_string(), false,
                                         vec!(Input::new(1, &None, false)),
                                         0,
                                         &vec!());
        function.init_inputs(true);
        function.write_input(0, json!(1));
        assert_eq!(function.take_input_set().remove(0).remove(0), json!(1));
    }

    #[test]
    fn cannot_send_input_if_full() {
        let mut function = Function::new("test".to_string(),
                                         "/context/test".to_string(),
                                         "/test".to_string(), false,
                                         vec!(Input::new(1, &None, false)),
                                         0,
                                         &vec!());
        function.init_inputs(true);
        function.write_input(0, json!(1)); // success
        function.write_input(0, json!(2)); // fail
        assert_eq!(function.take_input_set().remove(0).remove(0), json!(1));
    }
}
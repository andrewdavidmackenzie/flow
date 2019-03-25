use implementation::Implementation;
use implementation::RunAgain;
use input::{Input, InputInitializer};
use serde_json::Value as JsonValue;
use std::sync::Arc;
#[cfg(feature = "debugger")]
use std::fmt;

#[derive(Deserialize, Serialize)]
pub struct Process {
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
    #[serde(default = "Process::default_implementation")]
    implementation: Arc<Implementation>,
}

struct ImplementationNotFound;

impl Implementation for ImplementationNotFound {
    fn run(&self, _inputs: Vec<Vec<JsonValue>>) -> (Option<JsonValue>, RunAgain) {
        error!("Implementation not found");
        (None, false)
    }
}

#[cfg(feature = "debugger")]
impl fmt::Display for Process {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Process #{} '{}'\n", self.id, self.name)?;
        for (number, input) in self.inputs.iter().enumerate() {
            if input.is_empty() {
                write!(f, "\tInput #{}: empty\n", number)?;
            } else {
                write!(f, "\tInput #{}: {}\n", number, input)?;
            }
        }
        for output_route in &self.output_routes {
            write!(f, "\tOutput route '{}' -> {}:{}\n", output_route.0, output_route.1, output_route.2)?;
        }
        write!(f, "")
    }
}

impl Process {
    pub fn new(name: String,
               route: String,
               implementation_source: String,
               process_inputs: Vec<(usize, Option<InputInitializer>)>,
               id: usize,
               output_routes: Vec<(String, usize, usize)>) -> Process {
        let implementation = Process::default_implementation();

        let mut process = Process {
            name,
            route,
            id,
            implementation_source,
            implementation,
            output_routes,
            inputs: Vec::with_capacity(process_inputs.len()),
        };

        process.setup_inputs(process_inputs);

        process
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
        Arc::new(super::process::ImplementationNotFound {})
    }

    // Create the set of inputs, each with appropriate depth
    pub fn setup_inputs(&mut self, inputs: Vec<(usize, Option<InputInitializer>)>) {
        for input in inputs {
            self.inputs.push(Input::new(input.0, input.1));
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn id(&self) -> usize {
        self.id
    }

    /*
        If an initial value is defined then write it to the current value.
        Return true if ready to run as all inputs (single in this case) are satisfied.
    */
    pub fn init(&mut self) -> bool {
        // initialize any inputs that have initial values
        for mut input in &mut self.inputs {
            input.init(true);
        }

        self.can_run()
    }

    /*
        If any input of the process is initialized as a Constant, then refresh the input from the
        Constant Initializer

        // TODO look at this by instead never "taking" the input value away when dispatching
    */
    pub fn refresh_constant_inputs(&mut self) {
        for mut input in &mut self.inputs {
            input.init(false);
        }
    }

    pub fn implementation_source(&self) -> &str {
        &self.implementation_source
    }

    pub fn write_input(&mut self, input_number: usize, input_value: JsonValue) {
        if !self.inputs[input_number].full() {
            self.inputs[input_number].push(input_value);
        } else {
            error!("\t\t\tProcess #{} '{}' Input overflow on input number {}", self.id(), self.name(), input_number);
        }
    }

    pub fn output_destinations(&self) -> Vec<(String, usize, usize)> {
        self.output_routes.clone()
    }

    // TODO change to just return a reference to Implementation, doesn't need to be ref counted?
    pub fn get_implementation(&self) -> Arc<Implementation> {
        self.implementation.clone()
    }

    pub fn set_implementation(&mut self, implementation: Arc<Implementation>) {
        self.implementation = implementation;
    }

    pub fn input_full(&self, input_number: usize) -> bool {
        self.inputs[input_number].full()
    }

    // responds true if all inputs have been satisfied and this process can be run - false otherwise
    pub fn can_run(&self) -> bool {
        for input in &self.inputs {
            if !input.full() {
                return false;
            }
        }

        return true;
    }

    pub fn get_inputs(&self) -> &Vec<Input> {
        &self.inputs
    }

    pub fn get_input_values(&mut self) -> Vec<Vec<JsonValue>> {
        let mut input_values: Vec<Vec<JsonValue>> = Vec::new();
        for input_value in &mut self.inputs {
            input_values.push(input_value.take());
        }
        input_values
    }
}

#[cfg(test)]
mod test {
    use serde_json::value::Value as JsonValue;
    use super::Process;

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

    #[test]
    fn can_send_input_if_empty() {
        let mut process = Process::new("test".to_string(),
                                       "/context/test".to_string(),
                                       "/test".to_string(), vec!((1, None)), 0,
                                       vec!());
        process.init();
        process.write_input(0, json!(1));
        assert_eq!(process.get_input_values().remove(0).remove(0), json!(1));
    }

    #[test]
    fn cannot_send_input_if_full() {
        let mut process = Process::new("test".to_string(),
                                       "/context/test".to_string(),
                                       "/test".to_string(), vec!((1, None)), 0,
                                       vec!());
        process.init();
        process.write_input(0, json!(1)); // success
        process.write_input(0, json!(2)); // fail
        assert_eq!(process.get_input_values().remove(0).remove(0), json!(1));
    }
}
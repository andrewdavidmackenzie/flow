#[cfg(feature = "debugger")]
use std::fmt;
use std::sync::Arc;

use flow_impl::{Implementation, RunAgain};
use log::{debug, error, trace};
use serde_derive::{Deserialize, Serialize};
use serde_json::json;
use serde_json::Value;

use crate::input::Input;
use crate::output_connection::OutputConnection;

#[derive(Deserialize, Serialize, Clone)]
/// `Function` contains all the information needed about a fubction and its implementation
/// to be able to execute a flow using it.
pub struct Function {
    #[cfg(feature = "debugger")]
    #[serde(default, skip_serializing_if = "String::is_empty")]
    name: String,

    #[cfg(feature = "debugger")]
    #[serde(default, skip_serializing_if = "String::is_empty")]
    route: String,

    /// The unique `id` of this function at run-time
    id: usize,

    /// The unique id of the flow this function was in at definition time
    flow_id: usize,

    implementation_location: String,

    // TODO skip serializing this, if the vector ONLY contains objects that can be serialized
    // to "{}" and hence contain no info. I think the number of inputs is not needed?
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    inputs: Vec<Input>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    output_routes: Vec<OutputConnection>,

    #[serde(skip)]
    #[serde(default = "Function::default_implementation")]
    implementation: Arc<dyn Implementation>,
}

#[derive(Debug)]
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
        write!(f, "Function #{}", self.id)?;
        if !self.name.is_empty() {
            write!(f, " '{}'", self.name)?;
        }

        if !self.route.is_empty() {
            write!(f, " @ route '{}'\n", self.route)?;
        }

        for (number, input) in self.inputs.iter().enumerate() {
            if input.is_empty() {
                write!(f, "\tInput :{} is empty\n", number)?;
            } else {
                write!(f, "\tInput :{} has value '{}'\n", number, input)?;
            }
        }
        for output_route in &self.output_routes {
            write!(f, "\t{}\n", output_route)?;
        }
        write!(f, "")
    }
}

impl Function {
    /// Create a new `fubction` with the specified `name`, `route`, `implemenation` etc.
    /// This only needs to be used by compilers or IDE generating `manifests` with functions
    /// The library `flowrlib` just deserializes them from the `manifest`
    /// The Vector of outputs:
    /// Output sub-path (or ""), destination function id, destination function io number, Optional path of destination
    pub fn new(name: String,
               route: String,
               implementation_location: String,
               inputs: Vec<Input>,
               id: usize,
               flow_id: usize,
               output_routes: &Vec<OutputConnection>,
               include_destination_routes: bool) -> Function {
        let mut routes = (*output_routes).clone();

        // Remove destination routes if not wanted
        if !include_destination_routes {
            for mut r in &mut routes {
                r.route = None;
            }
        }

        Function {
            name,
            route,
            id,
            flow_id,
            implementation_location,
            implementation: Function::default_implementation(),
            output_routes: routes,
            inputs,
        }
    }

    #[cfg(feature = "debugger")]
    /// Reset a `Function` to initial state. Used by a debugger at run-time to reset a function
    /// as part of a whole flow reset to run it again.
    pub fn reset(&mut self) {
        for input in &mut self.inputs {
            input.reset();
        }
    }

    /// A default `Function` - used in deserialization of a `Manifest`
    pub fn default_implementation() -> Arc<dyn Implementation> {
        Arc::new(super::function::ImplementationNotFound {})
    }

    /// Accessor for a `Functions` `name`
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Accessor for a `Functions` `id`
    pub fn id(&self) -> usize {
        self.id
    }

    /// Accessor for a `Functions` `flow_id`
    pub fn get_flow_id(&self) -> usize {
        self.flow_id
    }

    /// Initialize all of a `Functions` `Inputs` - as they may have initializers that need running
    pub fn init_inputs(&mut self, first_time: bool) -> Vec<usize> {
        let mut refilled = vec!();
        for (io_number, input) in &mut self.inputs.iter_mut().enumerate() {
            if input.is_empty() {
                if input.init(first_time) {
                    trace!("\t\tInput #{}:{} set from initializer", self.id, io_number);
                    refilled.push(io_number);
                }
            }
        }
        refilled
    }

    /// Accessor for a `Functions` `implementation_location`
    pub fn implementation_location(&self) -> &str {
        &self.implementation_location
    }

    /// write a value to a `Functions` input -
    /// The value being written maybe an Array of values, in which case if the destination input does
    /// not accept Array, then iterate over the contents of the array and send each one to the
    /// input individually
    pub fn write_input(&mut self, input_number: usize, input_value: &Value) {
        let input = &mut self.inputs[input_number];
        if input_value.is_array() {
            // Serialize Array value into the non-Array input
            if !input.is_array {
                debug!("\t\tSerializing Array value to non-Array input");
                input.push_array(input_value.as_array().unwrap().iter());
            } else {
                // Send Array value to the Array input
                input.push(input_value.clone());
            }
        } else {
            if input.is_array {
                // Send Non-Array value to the Array input
                input.push(json!([input_value]));
            } else {
                // Send Non-Array value to Non-Array input
                input.push(input_value.clone());
            }
        }
    }

    /// Accessor for a `Functions` `output_routes` field
    pub fn output_destinations(&self) -> &Vec<OutputConnection> {
        &self.output_routes
    }

    /// Get a clone of the `Functions` `implementation`
    pub fn get_implementation(&self) -> Arc<dyn Implementation> {
        self.implementation.clone()
    }

    /// Set a `Functions` `implementation`
    pub fn set_implementation(&mut self, implementation: Arc<dyn Implementation>) {
        self.implementation = implementation;
    }

    /// Determine if the `Functions` `input` number `input_number` is full or not
    pub fn input_full(&self, input_number: usize) -> bool {
        self.inputs[input_number].full()
    }

    /// Determine if all of the `Functions` `inputs` are full and this function can be run
    pub fn inputs_full(&self) -> bool {
        for input in &self.inputs {
            if !input.full() {
                return false;
            }
        }

        return true;
    }

    #[cfg(feature = "debugger")]
    /// Inpect the values of the `inputs` of a feature. Only used by the `debugger` feature
    pub fn inputs(&self) -> &Vec<Input> {
        &self.inputs
    }

    /// Read the values from the inputs and return them for use in executing the function
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
    use std::sync::Arc;

    use flow_impl::Implementation;
    use serde_json::json;
    use serde_json::value::Value;

    use crate::input::Input;
    use crate::output_connection::OutputConnection;

    use super::Function;
    use super::ImplementationNotFound;

    /*************** Below are tests for basic json.pointer functionality *************************/

    #[test]
    fn destructure_output_base_route() {
        let json = json!("simple");
        assert_eq!("simple", json.pointer("").unwrap(), "json pointer functionality not working!");
    }

    #[test]
    fn destructure_json_value() {
        let json: Value = json!({ "sub_route": "sub_output" });
        assert_eq!("sub_output", json.pointer("/sub_route").unwrap(), "json pointer functionality not working!");
    }

    #[test]
    fn access_array_elements() {
        let args: Vec<&str> = vec!("arg0", "arg1", "arg2");
        let json = json!(args);
        assert_eq!("arg0", json.pointer("/0").unwrap(), "json pointer array indexing functionality not working!");
        assert_eq!("arg1", json.pointer("/1").unwrap(), "json pointer array indexing functionality not working!");
    }

    /*************** Below are tests for inputs with depth = 1 ***********************/

    #[test]
    fn can_send_simple_object() {
        let mut function = Function::new("test".to_string(),
                                         "/test".to_string(),
                                         "/test".to_string(),
                                         vec!(Input::new(1, &None, false)),
                                         0, 0,
                                         &vec!(), false);
        function.init_inputs(true);
        function.write_input(0, &json!(1));
        assert_eq!(json!(1), function.take_input_set().remove(0).remove(0),
                   "Value from input set wasn't what was expected");
    }

    #[test]
    fn can_send_array_object() {
        let mut function = Function::new("test".to_string(),
                                         "/test".to_string(),
                                         "/test".to_string(),
                                         vec!(Input::new(1, &None, true)),
                                         0, 0,
                                         &vec!(), false);
        function.init_inputs(true);
        function.write_input(0, &json!([1, 2]));
        assert_eq!(json!([1, 2]), function.take_input_set().remove(0).remove(0),
                   "Value from input set wasn't what was expected");
    }

    #[test]
    fn can_send_simple_object_to_array_input() {
        let mut function = Function::new("test".to_string(),
                                         "/test".to_string(),
                                         "/test".to_string(),
                                         vec!(Input::new(1, &None, true)),
                                         0, 0,
                                         &vec!(), false);
        function.init_inputs(true);
        function.write_input(0, &json!(1));
        assert_eq!(vec!(json!([1])), function.take_input_set().remove(0),
                   "Value from input set wasn't what was expected");
    }

    #[test]
    fn can_send_array_to_simple_object_depth_1() {
        let mut function = Function::new("test".to_string(),
                                         "/test".to_string(),
                                         "/test".to_string(),
                                         vec!(Input::new(1, &None, false)),
                                         0, 0,
                                         &vec!(), false);
        function.init_inputs(true);
        function.write_input(0, &json!([1, 2]));
        assert_eq!(vec!(json!(1)), function.take_input_set().remove(0),
                   "Value from input set wasn't what was expected");
    }

    #[test]
    fn second_value_overwrites_on_oversend() {
        let mut function = Function::new("test".to_string(),
                                         "/test".to_string(),
                                         "/test".to_string(),
                                         vec!(Input::new(1, &None, false)),
                                         0, 0,
                                         &vec!(), false);
        function.init_inputs(true);
        function.write_input(0, &json!(1));
        function.write_input(0, &json!(2));
        assert_eq!(json!(2), function.take_input_set().remove(0).remove(0),
                   "Value from input set wasn't what was expected");
    }

    #[test]
    #[should_panic]
    fn cannot_take_input_set_if_not_full() {
        let mut function = Function::new("test".to_string(),
                                         "/test".to_string(),
                                         "/test".to_string(),
                                         vec!(Input::new(1, &None, false)),
                                         0, 0,
                                         &vec!(), false);
        function.init_inputs(true);
        function.take_input_set().remove(0);
    }

    /*************** Below are tests for inputs with depth > 1 ***********************/

    #[test]
    fn can_send_array_to_simple_object_depth_2() {
        let mut function = Function::new("test".to_string(),
                                         "/test".to_string(),
                                         "/test".to_string(),
                                         vec!(Input::new(2, &None, false)),
                                         0, 0,
                                         &vec!(), false);
        function.init_inputs(true);
        function.write_input(0, &json!([1, 2]));
        assert_eq!(vec!(json!(1), json!(2)), function.take_input_set().remove(0),
                   "Value from input set wasn't what was expected");
    }

    #[test]
    fn can_send_simple_object_when_depth_more_than_1() {
        let mut function = Function::new("test".to_string(),
                                         "/test".to_string(),
                                         "/test".to_string(),
                                         vec!(Input::new(2, &None, false)),
                                         0, 0,
                                         &vec!(), false);
        function.init_inputs(true);
        function.write_input(0, &json!(1));
        function.write_input(0, &json!(2));
        assert_eq!(vec!(json!(1), json!(2)), function.take_input_set().remove(0),
                   "Value from input set wasn't the array of numbers expected");
    }

    #[test]
    fn can_send_array_objects_when_input_depth_more_than_1() {
        let mut function = Function::new("test".to_string(),
                                         "/test".to_string(),
                                         "/test".to_string(),
                                         vec!(Input::new(2, &None, true)),
                                         0, 0,
                                         &vec!(), false);
        function.init_inputs(true);
        function.write_input(0, &json!([1, 2]));
        function.write_input(0, &json!([3, 4]));
        assert_eq!(vec!(json!([1, 2]), json!([3, 4])), function.take_input_set().remove(0),
                   "Value from input set wasn't what was expected");
    }

    #[test]
    #[should_panic]
    fn cannot_take_input_set_if_not_full_depth_2() {
        let mut function = Function::new("test".to_string(),
                                         "/test".to_string(),
                                         "/test".to_string(),
                                         vec!(Input::new(2, &None, false)),
                                         0, 0,
                                         &vec!(), false);
        function.init_inputs(true);
        function.write_input(0, &json!(1));
        function.take_input_set().remove(0);
    }

    fn test_function() -> Function {
        let out_conn = OutputConnection::new("/other/input/1".to_string(),
                                             1, 1, 0, None);
        Function::new("test".to_string(),
                      "/test".to_string(),
                      "/implementation".to_string(),
                      vec!(Input::new(2, &None, false)),
                      1, 0,
                      &vec!(out_conn), false)
    }

    #[cfg(feature = "debugger")]
    #[test]
    fn debugger_can_inspect_non_full_input() {
        let mut function = test_function();
        function.init_inputs(true);
        function.write_input(0, &json!(1));
        assert_eq!(function.inputs().len(), 1, "Could not read incomplete input set");
    }

    #[test]
    fn call_implementation_not_found_panics() {
        let inf = ImplementationNotFound {};
        assert_eq!((None, false), inf.run(vec!()), "ImplementationNotFound should return (None, false)");
    }

    #[cfg(feature = "debugger")]
    #[test]
    fn can_display_function() {
        let function = test_function();
        let _ = format!("{}", function);
    }

    #[cfg(feature = "debugger")]
    #[test]
    fn can_display_function_with_inputs() {
        let output_route = OutputConnection::new("/other/input/1".to_string(),
                                                 1, 1, 0, None);
        let mut function = Function::new("test".to_string(),
                                         "/test".to_string(),
                                         "/test".to_string(),
                                         vec!(Input::new(2, &None, false)),
                                         0, 0,
                                         &vec!(output_route.clone()), false);
        function.init_inputs(true);
        function.write_input(0, &json!(1));
        let _ = format!("{}", function);
        assert_eq!(&vec!(output_route), function.output_destinations(), "output routes not as originally set");
    }

    #[test]
    fn can_get_function_name_and_id_and_location() {
        let function = test_function();
        assert_eq!("test".to_string(), function.name());
        assert_eq!(1, function.id());
        assert_eq!("/implementation", function.implementation_location());
    }

    #[test]
    fn can_set_and_get_implementation() {
        let mut function = test_function();
        let inf = Arc::new(ImplementationNotFound {});
        function.set_implementation(inf);
        let _ = function.get_implementation();
    }
}
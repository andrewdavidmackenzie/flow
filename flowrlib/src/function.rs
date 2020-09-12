#[cfg(feature = "debugger")]
use std::fmt;
use std::sync::Arc;

use flow_impl::{Implementation, RunAgain};
use log::{error, trace};
use serde_derive::{Deserialize, Serialize};
use serde_json::Value;

use crate::errors::*;
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
    fn run(&self, _inputs: &[Value]) -> (Option<Value>, RunAgain) {
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
            writeln!(f, " @ route '{}'", self.route)?;
        }

        for (number, input) in self.inputs.iter().enumerate() {
            if input.is_empty() {
                writeln!(f, "\tInput :{} is empty", number)?;
            } else {
                writeln!(f, "\tInput :{} has value '{}'", number, input)?;
            }
        }
        for output_route in &self.output_routes {
            writeln!(f, "\t{}", output_route)?;
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
    #[allow(clippy::too_many_arguments)]
    pub fn new(
               #[cfg(feature = "debugger")]
               name: String,
               #[cfg(feature = "debugger")]
               route: String,
               implementation_location: String,
               inputs: Vec<Input>,
               id: usize,
               flow_id: usize,
               output_routes: &[OutputConnection],
               include_destination_routes: bool) -> Self {
        let mut routes = output_routes.to_vec();

        // Remove destination routes if not wanted
        if !include_destination_routes {
            for mut r in &mut routes {
                r.route = None;
            }
        }

        Function {
            #[cfg(feature = "debugger")]
            name,
            #[cfg(feature = "debugger")]
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
    #[cfg(feature = "debugger")]
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
    pub fn init_inputs(&mut self, first_time: bool) {
        for (io_number, input) in &mut self.inputs.iter_mut().enumerate() {
            if input.is_empty() && input.init(first_time, io_number) {
                trace!("\t\tInput #{}:{} set from initializer", self.id, io_number);
            }
        }
    }

    /// Accessor for a `Functions` `implementation_location`
    pub fn implementation_location(&self) -> &str {
        &self.implementation_location
    }

    /// write a value to a `Function`'s input
    pub fn send(&mut self, input_number: usize, value: &Value) {
        let input = &mut self.inputs[input_number];
        input.push(value.clone());
    }

    /// write an array of values to a `Function`'s input
    pub fn send_iter(&mut self, input_number: usize, value: &Value) {
        let input = &mut self.inputs[input_number];
        input.push_array(value.as_array().unwrap().iter());
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
            if input.is_empty() {
                return false;
            }
        }

        true
    }

    #[cfg(feature = "debugger")]
    /// Inpect the values of the `inputs` of a feature. Only used by the `debugger` feature
    pub fn inputs(&self) -> &Vec<Input> {
        &self.inputs
    }

    /// Read the values from the inputs and return them for use in executing the function
    pub fn take_input_set(&mut self) -> Result<Vec<Value>> {
        let id = self.id;
        let mut input_set: Vec<Value> = Vec::new();
        for input in &mut self.inputs {
            let input_value = input.take()
                .chain_err(|| format!("Error taking from input of Function #{}", id))?;
            input_set.push(input_value);
        }
        Ok(input_set)
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

    #[test]
    fn can_send_simple_object() {
        let mut function = Function::new(
                                    #[cfg(feature = "debugger")]
                                          "test".to_string(),
                                         #[cfg(feature = "debugger")]
                                         "/test".to_string(),
                                         "/test".to_string(),
                                         vec!(Input::new(&None)),
                                         0, 0,
                                         &[], false);
        function.init_inputs(true);
        function.send(0, &json!(1));
        assert_eq!(json!(1), function.take_input_set().unwrap().remove(0),
                   "Value from input set wasn't what was expected");
    }

    #[test]
    fn can_send_array_object() {
        let mut function = Function::new(
                                    #[cfg(feature = "debugger")]
                                        "test".to_string(),
                                         #[cfg(feature = "debugger")]
                                         "/test".to_string(),
                                         "/test".to_string(),
                                         // vec!(Input::new(1, &None, true, false)),
                                         vec!(Input::new(&None)),
                                         0, 0,
                                         &[], false);
        function.init_inputs(true);
        function.send(0, &json!([1, 2]));
        assert_eq!(json!([1, 2]), function.take_input_set().unwrap().remove(0),
                   "Value from input set wasn't what was expected");
    }

    #[test]
    fn test_array_to_non_array() {
        let mut function = Function::new(
            #[cfg(feature = "debugger")]
                "test".to_string(),
            #[cfg(feature = "debugger")]
                "/test".to_string(),
            "/test".to_string(),
            vec!(Input::new(&None)),
            0, 0,
            &[], false);
        function.init_inputs(true);
        function.send(0, &json!([1, 2]));
        assert_eq!(function.take_input_set().unwrap().remove(0), json!([1, 2]),
                   "Value from input set wasn't what was expected");
    }

    fn test_function() -> Function {
        let out_conn = OutputConnection::new("/other/input/1".to_string(),
                                             1, 1, 0, 0, false, None);
        Function::new(
            #[cfg(feature = "debugger")]
                    "test".to_string(),
            #[cfg(feature = "debugger")]
                        "/test".to_string(),
                      "/implementation".to_string(),
                      vec!(Input::new(&None)),
                      1, 0,
                      &[out_conn], false)
    }

    #[cfg(feature = "debugger")]
    #[test]
    fn debugger_can_inspect_non_full_input() {
        let mut function = test_function();
        function.init_inputs(true);
        function.send(0, &json!(1));
        assert_eq!(function.inputs().len(), 1, "Could not read incomplete input set");
    }

    #[test]
    fn implementation_not_found() {
        let inf = ImplementationNotFound {};
        assert_eq!((None, false), inf.run(&[]), "ImplementationNotFound should return (None, false)");
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
                                                 1, 1, 0, 0, false, None);
        let mut function = Function::new(
            #[cfg(feature = "debugger")]
                                        "test".to_string(),
            #[cfg(feature = "debugger")]
                                         "/test".to_string(),
                                         "/test".to_string(),
                                         vec!(Input::new(&None)),
                                         0, 0,
                                         &[output_route.clone()], false);
        function.init_inputs(true);
        function.send(0, &json!(1));
        let _ = format!("{}", function);
        assert_eq!(&vec!(output_route), function.output_destinations(), "output routes not as originally set");
    }

    #[test]
    fn can_get_function_name_and_id_and_location() {
        let function = test_function();
        #[cfg(feature = "debugger")]
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
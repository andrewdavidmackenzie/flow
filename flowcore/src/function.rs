#[cfg(feature = "debugger")]
use std::fmt;
use std::sync::Arc;

use log::{error, trace};
use serde_derive::{Deserialize, Serialize};
use serde_json::Value;

use crate::errors::*;
use crate::input::Input;
use crate::output_connection::OutputConnection;
use crate::{Implementation, RunAgain};

#[derive(Deserialize, Serialize, Clone)]
/// `Function` contains all the information needed about a function and its implementation
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

    /// Implementation location can be a "lib://lib_name/path/to/implementation" reference
    /// or a path relative to the manifest location where a supplied implementation file can be found
    implementation_location: String,

    // TODO skip serializing this, if the vector ONLY contains objects that can be serialized
    // to "{}" and hence contain no info. I think the number of inputs is not needed?
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    inputs: Vec<Input>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    output_connections: Vec<OutputConnection>,

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
        write!(f, "Function #{}({})", self.id, self.flow_id)?;

        if !self.name.is_empty() {
            write!(f, " '{}'", self.name)?;
        }

        if !self.route.is_empty() {
            writeln!(f, " @ '{}'", self.route)?;
        }

        if !self.implementation_location.is_empty() {
            writeln!(
                f,
                "\tImplementation Location: '{}'",
                self.implementation_location
            )?;
        }

        for (number, input) in self.inputs.iter().enumerate() {
            writeln!(f, "\tInput:{} {}", number, input)?;
        }

        for output_route in &self.output_connections {
            writeln!(f, "\t{}", output_route)?;
        }

        Ok(())
    }
}

impl Function {
    /// Create a new `function` with the specified `name`, `route`, `implementation` etc.
    /// This only needs to be used by compilers or IDE generating `manifests` with functions
    /// The library `flowrlib` just deserializes them from the `manifest`
    /// The Vector of outputs:
    /// Output sub-path (or ""), destination function id, destination function io number, Optional path of destination
    #[allow(clippy::too_many_arguments)]
    pub fn new<
        #[cfg(feature = "debugger")] N: Into<String>,
        #[cfg(feature = "debugger")] R: Into<String>,
        I: Into<String>,
    >(
        #[cfg(feature = "debugger")] name: N,
        #[cfg(feature = "debugger")] route: R,
        implementation_location: I,
        inputs: Vec<Input>,
        id: usize,
        flow_id: usize,
        output_connections: &[OutputConnection],
        include_destination_routes: bool,
    ) -> Self {
        let mut routes = output_connections.to_vec();

        // Remove destination routes if not wanted
        if !include_destination_routes {
            for mut r in &mut routes {
                r.route = String::default();
            }
        }

        Function {
            #[cfg(feature = "debugger")]
            name: name.into(),
            #[cfg(feature = "debugger")]
            route: route.into(),
            id,
            flow_id,
            implementation_location: implementation_location.into(),
            implementation: Function::default_implementation(),
            output_connections: routes,
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
    pub fn init_inputs(&mut self, first_time: bool) -> bool {
        let mut inputs_initialized = false;
        for (io_number, input) in &mut self.inputs.iter_mut().enumerate() {
            if input.count() == 0 && input.init(first_time, io_number) {
                trace!("\t\tInput #{}:{} set from initializer", self.id, io_number);
                inputs_initialized = true;
            }
        }
        inputs_initialized
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
    pub fn send_iter(&mut self, input_number: usize, array: &[Value]) {
        let input = &mut self.inputs[input_number];
        input.push_array(array.iter());
    }

    /// Accessor for a `Functions` `output_connections` field
    pub fn get_output_connections(&self) -> &Vec<OutputConnection> {
        &self.output_connections
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
    pub fn input_count(&self, input_number: usize) -> usize {
        self.inputs[input_number].count()
    }

    /// Returns how many inputs sets are available across all the `Functions` `inputs`
    pub fn input_set_count(&self) -> usize {
        let mut num_input_sets = usize::MAX;

        for input in &self.inputs {
            num_input_sets = std::cmp::min(num_input_sets, input.count());
        }

        num_input_sets
    }

    /// Inspect the values of the `inputs` of a function
    #[cfg(feature = "debugger")]
    pub fn inputs(&self) -> &Vec<Input> {
        &self.inputs
    }

    /// Inspect the value of the `input` of a feature.
    #[cfg(feature = "debugger")]
    pub fn input(&self, id: usize) -> &Input {
        &self.inputs[id]
    }

    /// Read the values from the inputs and return them for use in executing the function
    pub fn take_input_set(&mut self) -> Result<Vec<Value>> {
        let mut input_set: Vec<Value> = Vec::new();
        for input in &mut self.inputs {
            input_set.push(input.take()?);
        }
        Ok(input_set)
    }
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use serde_json::json;
    use serde_json::value::Value;

    use crate::input::Input;
    use crate::output_connection::OutputConnection;
    use crate::output_connection::Source::Output;
    use crate::Implementation;

    use super::Function;
    use super::ImplementationNotFound;

    /*************** Below are tests for basic json.pointer functionality *************************/

    #[test]
    fn destructure_output_base_route() {
        let json = json!("simple");
        assert_eq!(
            "simple",
            json.pointer("").expect("Couldn't get root element"),
            "json pointer functionality not working!"
        );
    }

    #[test]
    fn destructure_json_value() {
        let json: Value = json!({ "sub_route": "sub_output" });
        assert_eq!(
            "sub_output",
            json.pointer("/sub_route").expect("Couldn't get route"),
            "json pointer functionality not working!"
        );
    }

    #[test]
    fn access_array_elements() {
        let args: Vec<&str> = vec!["arg0", "arg1", "arg2"];
        let json = json!(args);
        assert_eq!(
            "arg0",
            json.pointer("/0").expect("Couldn't get /0 route"),
            "json pointer array indexing functionality not working!"
        );
        assert_eq!(
            "arg1",
            json.pointer("/1").expect("Couldn't get /1 route"),
            "json pointer array indexing functionality not working!"
        );
    }

    #[test]
    fn can_send_simple_object() {
        let mut function = test_function();
        function.init_inputs(true);
        function.send(0, &json!(1));
        assert_eq!(
            json!(1),
            function
                .take_input_set()
                .expect("Couldn't get input set")
                .remove(0),
            "Value from input set wasn't what was expected"
        );
    }

    #[test]
    fn can_send_array_object() {
        let mut function = test_function();
        function.init_inputs(true);
        function.send(0, &json!([1, 2]));
        assert_eq!(
            json!([1, 2]),
            function
                .take_input_set()
                .expect("Couldn't get input set")
                .remove(0),
            "Value from input set wasn't what was expected"
        );
    }

    #[test]
    fn test_array_to_non_array() {
        let mut function = test_function();
        function.init_inputs(true);
        function.send(0, &json!([1, 2]));
        assert_eq!(
            function
                .take_input_set()
                .expect("Couldn't get input set")
                .remove(0),
            json!([1, 2]),
            "Value from input set wasn't what was expected"
        );
    }

    fn test_function() -> Function {
        let out_conn = OutputConnection::new(
            Output("/other/input/1".into()),
            1,
            1,
            0,
            0,
            false,
            String::default(),
            #[cfg(feature = "debugger")]
            String::default(),
        );
        Function::new(
            #[cfg(feature = "debugger")]
            "test",
            #[cfg(feature = "debugger")]
            "/test",
            "file://fake/implementation",
            vec![Input::new(&None)],
            1,
            0,
            &[out_conn],
            false,
        )
    }

    #[cfg(feature = "debugger")]
    #[test]
    fn debugger_can_inspect_non_full_input() {
        let mut function = test_function();
        function.init_inputs(true);
        function.send(0, &json!(1));
        assert_eq!(
            function.inputs().len(),
            1,
            "Could not read incomplete input set"
        );
    }

    #[test]
    fn implementation_not_found() {
        let inf = ImplementationNotFound {};
        assert_eq!(
            (None, false),
            inf.run(&[]),
            "ImplementationNotFound should return (None, false)"
        );
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
        let output_route = OutputConnection::new(
            Output("/other/input/1".into()),
            1,
            1,
            0,
            0,
            false,
            String::default(),
            #[cfg(feature = "debugger")]
            String::default(),
        );
        let mut function = Function::new(
            #[cfg(feature = "debugger")]
            "test",
            #[cfg(feature = "debugger")]
            "/test",
            "file://fake/test",
            vec![Input::new(&None)],
            0,
            0,
            &[output_route.clone()],
            false,
        );
        function.init_inputs(true);
        function.send(0, &json!(1));
        let _ = format!("{}", function);
        assert_eq!(
            &vec!(output_route),
            function.get_output_connections(),
            "output routes not as originally set"
        );
    }

    #[test]
    fn can_get_function_name_and_id_and_location() {
        let function = test_function();
        #[cfg(feature = "debugger")]
        assert_eq!("test".to_string(), function.name());
        assert_eq!(1, function.id());
        assert_eq!(
            "file://fake/implementation",
            function.implementation_location()
        );
    }

    #[test]
    fn can_set_and_get_implementation() {
        let mut function = test_function();
        let inf = Arc::new(ImplementationNotFound {});
        function.set_implementation(inf);
        let _ = function.get_implementation();
    }
}

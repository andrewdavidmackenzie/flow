#[cfg(feature = "debugger")]
use std::fmt;
use std::sync::Arc;

use log::{debug, error};
use serde_derive::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{Implementation, RunAgain};
use crate::errors::*;
use crate::model::input::Input;
use crate::model::output_connection::OutputConnection;

#[derive(Deserialize, Serialize, Clone)]
/// `RuntimeFunction` contains all the information needed about a function and its implementation
/// to be able to execute a flow using it.
pub struct RuntimeFunction {
    #[cfg(feature = "debugger")]
    #[serde(default, skip_serializing_if = "String::is_empty")]
    name: String,

    #[cfg(feature = "debugger")]
    #[serde(default, skip_serializing_if = "String::is_empty")]
    route: String,

    /// The unique `function_id` of this function at run-time
    function_id: usize,

    /// The unique id of the flow this function was in at definition time
    flow_id: usize,

    /// Implementation location can be a "lib://lib_name/path/to/implementation" reference
    /// or a "context://stdio/stdout" context function reference
    /// or a path relative to the manifest location where a supplied implementation file can be found
    implementation_location: String,

    // TODO skip serializing this, if the vector ONLY contains objects that can be serialized
    // to "{}" and hence contain no info. I think the number of inputs is not needed?
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    inputs: Vec<Input>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    output_connections: Vec<OutputConnection>,

    #[serde(skip)]
    #[serde(default = "RuntimeFunction::default_implementation")]
    implementation: Arc<dyn Implementation>,
}

#[derive(Debug)]
struct ImplementationNotFound;

impl Implementation for ImplementationNotFound {
    fn run(&self, _inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
        bail!("Implementation not found");
    }
}

#[cfg(feature = "debugger")]
impl fmt::Display for RuntimeFunction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Function #{}({})", self.function_id, self.flow_id)?;

        if !self.name.is_empty() {
            write!(f, " '{}'", self.name)?;
        }

        if !self.route.is_empty() {
            writeln!(f, " @ '{}'", self.route)?;
        }

        writeln!(f, "\t({})", self.implementation_location)?;

        for (number, input) in self.inputs.iter().enumerate() {
            writeln!(f, "\tInput:{number} {input}")?;
        }

        for output_route in &self.output_connections {
            writeln!(f, "\t{output_route}",)?;
        }

        Ok(())
    }
}

impl RuntimeFunction {
    /// Create a new `RuntimeFunction` with the specified `name`, `route`, `implementation` etc.
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
        let mut connections = output_connections.to_vec();

        // Remove destination routes if not wanted
        if !include_destination_routes {
            for mut connection in &mut connections {
                connection.destination = String::default();
            }
        }

        RuntimeFunction {
            #[cfg(feature = "debugger")]
            name: name.into(),
            #[cfg(feature = "debugger")]
            route: route.into(),
            function_id: id,
            flow_id,
            implementation_location: implementation_location.into(),
            implementation: RuntimeFunction::default_implementation(),
            output_connections: connections,
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
        Arc::new(ImplementationNotFound {})
    }

    /// Accessor for a `RuntimeFunction` `name`
    #[cfg(feature = "debugger")]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Accessor for a `RuntimeFunction` `route`
    #[cfg(feature = "debugger")]
    pub fn route(&self) -> &str {
        &self.route
    }

    /// Accessor for a `RuntimeFunction` `id`
    pub fn id(&self) -> usize {
        self.function_id
    }

    /// Accessor for a `RuntimeFunction` `flow_id`
    pub fn get_flow_id(&self) -> usize {
        self.flow_id
    }

    /// Initialize all of a `RuntimeFunction` `Inputs` that have initializers on them
    pub fn init_inputs(&mut self, first_time: bool) {
        for (io_number, input) in &mut self.inputs.iter_mut().enumerate() {
            if input.init(first_time) {
                debug!("\tInitialized Input #{}:{io_number} in Flow #{}", self.function_id, self.flow_id);
            }
        }
    }

    /// Accessor for a `RuntimeFunction` `implementation_location`
    pub fn implementation_location(&self) -> &str {
        &self.implementation_location
    }

    // Take a json data value and return the array order for it
    fn array_order(value: &Value) -> i32 {
        match value {
            Value::Array(array) if !array.is_empty() => {
                if let Some(value) = array.get(0) {
                    1 + Self::array_order(value)
                } else {
                    1
                }
            },
            Value::Array(array) if array.is_empty() => 1,
            _ => 0,
        }
    }

    /// Do the necessary serialization of an array to values, or wrapping of a value into an array
    /// in order to convert the value on expected by the destination, if possible, send the value
    /// and return true. If the conversion cannot be done and no value is sent, return false.
    pub fn type_convert_and_send(&mut self,
        connection: &OutputConnection,
        value: &Value) -> bool {
        if connection.is_generic() {
            self.send(connection.destination_io_number, value);
        } else {
            match (
                (Self::array_order(value) - connection.destination_array_order),
                value,
            ) {
                (0, _) => self.send(connection.destination_io_number, value),
                (1, Value::Array(array)) => self.send_iter(connection.destination_io_number,
                                                           array),
                (2, Value::Array(array_2)) => {
                    for array in array_2.iter() {
                        if let Value::Array(sub_array) = array {
                            self.send_iter(connection.destination_io_number, sub_array)
                        }
                    }
                }
                (-1, _) => self.send(connection.destination_io_number, &json!([value])),
                (-2, _) => self.send(connection.destination_io_number, &json!([[value]])),
                _ => {
                    error!("Unable to handle difference in array order");
                    return false;
                },
            }
        }
        true // a value was sent!
    }

    // write a value to a `RuntimeFunction`'s `input`
    fn send(&mut self, input_number: usize, value: &Value) {
        self.inputs[input_number].push(value.clone());
    }

    // write an array of values to a `RuntimeFunction` `input`
    fn send_iter(&mut self, input_number: usize, array: &[Value]) {
        self.inputs[input_number].push_array(array.iter());
    }

    /// Accessor for a `RuntimeFunction` `output_connections` field
    pub fn get_output_connections(&self) -> &Vec<OutputConnection> {
        &self.output_connections
    }

    /// Get a clone of the `RuntimeFunction` `implementation`
    pub fn get_implementation(&self) -> Arc<dyn Implementation> {
        self.implementation.clone()
    }

    /// Set a `RuntimeFunction` `implementation`
    pub fn set_implementation(&mut self, implementation: Arc<dyn Implementation>) {
        self.implementation = implementation;
    }

    /// Determine if the `RuntimeFunction` `input` number `input_number` is full or not
    pub fn input_count(&self, input_number: usize) -> usize {
        self.inputs[input_number].count()
    }

    /// Returns how many inputs sets are available across all the `RuntimeFunction` `Inputs`
    /// NOTE: For Impure functions without inputs (that can always run and produce a value)
    /// this will return usize::MAX
    pub fn input_set_count(&self) -> usize {
        let mut num_input_sets = usize::MAX;

        for input in &self.inputs {
            num_input_sets = std::cmp::min(num_input_sets, input.count());
        }

        num_input_sets
    }

    /// Can this function run? Either because:
    ///     - it has input sets to allow it to run
    ///     - it has no inputs and so can always run
    pub fn can_run(&self) -> bool {
        self.inputs.is_empty() || self.input_set_count() > 0
    }

    /// Inspect the values of the `inputs` of a `RuntimeFunction`
    #[cfg(any(feature = "debugger", debug_assertions))]
    pub fn inputs(&self) -> &Vec<Input> {
        &self.inputs
    }

    /// Inspect the value of the `input` of a `RuntimeFunction`.
    #[cfg(feature = "debugger")]
    pub fn input(&self, id: usize) -> Option<&Input> {
        self.inputs.get(id)
    }

    /// Read the values from the inputs and return them for use in executing the `RuntimeFunction`
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

    use crate::Implementation;
    use crate::model::input::Input;
    use crate::model::output_connection::OutputConnection;
    use crate::model::output_connection::Source::Output;

    use super::ImplementationNotFound;
    use super::RuntimeFunction;

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
            "The value from input set wasn't what was expected"
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
            "The value from input set wasn't what was expected"
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
            "The value from input set wasn't what was expected"
        );
    }

    fn test_function() -> RuntimeFunction {
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
        RuntimeFunction::new(
            #[cfg(feature = "debugger")]
            "test",
            #[cfg(feature = "debugger")]
            "/test",
            "file://fake/implementation",
            vec![Input::new("", None, None)],
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
        assert!(inf.run(&[]).is_err());
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
        let mut function = RuntimeFunction::new(
            #[cfg(feature = "debugger")]
            "test",
            #[cfg(feature = "debugger")]
            "/test",
            "file://fake/test",
            vec![Input::new("", None, None)],
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

    mod misc {
        use serde_json::{json, Value};

        use crate::model::input::Input;
        use crate::model::output_connection::{OutputConnection, Source};
        use crate::model::runtime_function::RuntimeFunction;

        #[test]
        fn test_array_order_0() {
            let value = json!(1);
            assert_eq!(RuntimeFunction::array_order(&value), 0);
        }

        #[test]
        fn test_array_order_1_empty_array() {
            let value = json!([]);
            assert_eq!(RuntimeFunction::array_order(&value), 1);
        }

        #[test]
        fn test_array_order_1() {
            let value = json!([1, 2, 3]);
            assert_eq!(RuntimeFunction::array_order(&value), 1);
        }

        #[test]
        fn test_array_order_2() {
            let value = json!([[1, 2, 3], [2, 3, 4]]);
            assert_eq!(RuntimeFunction::array_order(&value), 2);
        }

        fn test_function() -> RuntimeFunction {
            RuntimeFunction::new(
                #[cfg(feature = "debugger")]
                    "test",
                #[cfg(feature = "debugger")]
                    "/test",
                "file://fake/test",
                vec![Input::new(
                    #[cfg(feature = "debugger")] "",
                    None, None)],
                0,
                0,
                &[],
                false,
            )
        }

        // Test type conversion and sending
        //                         |                   Destination
        //                         |Generic     Non-Array       Array       Array of Arrays
        // Value       Value order |    N/A         0               1       2      <---- Array Order
        //  Non-Array       (0)    |   send     (0) send        (-1) wrap   (-2) wrap in array of arrays
        //  Array           (1)    |   send     (1) iter        (0) send    (-1) wrap in array
        //  Array of Arrays (2)    |   send     (2) iter/iter   (1) iter    (0) send
        #[test]
        fn test_sending() {
            #[derive(Debug)]
            struct TestCase {
                value: Value,
                destination_is_generic: bool,
                destination_array_order: i32,
                value_expected: Value,
            }

            let test_cases = vec![
                // Column 0 test cases
                TestCase {
                    value: json!(1),
                    destination_is_generic: true,
                    destination_array_order: 0,
                    value_expected: json!(1),
                },
                TestCase {
                    value: json!([1]),
                    destination_is_generic: true,
                    destination_array_order: 0,
                    value_expected: json!([1]),
                },
                TestCase {
                    value: json!([[1, 2], [3, 4]]),
                    destination_is_generic: true,
                    destination_array_order: 0,
                    value_expected: json!([[1, 2], [3, 4]]),
                },
                // Column 1 Test Cases
                TestCase {
                    value: json!(1),
                    destination_is_generic: false,
                    destination_array_order: 0,
                    value_expected: json!(1),
                },
                TestCase {
                    value: json!([1, 2]),
                    destination_is_generic: false,
                    destination_array_order: 0,
                    value_expected: json!(1),
                },
                TestCase {
                    value: json!([[1, 2], [3, 4]]),
                    destination_is_generic: false,
                    destination_array_order: 0,
                    value_expected: json!(1),
                },
                // Column 2 Test Cases
                TestCase {
                    value: json!(1),
                    destination_is_generic: false,
                    destination_array_order: 1,
                    value_expected: json!([1]),
                },
                TestCase {
                    value: json!([1, 2]),
                    destination_is_generic: false,
                    destination_array_order: 1,
                    value_expected: json!([1, 2]),
                },
                TestCase {
                    value: json!([[1, 2], [3, 4]]),
                    destination_is_generic: false,
                    destination_array_order: 1,
                    value_expected: json!([1, 2]),
                },
                // Column 3 Test Cases
                TestCase {
                    value: json!(1),
                    destination_is_generic: false,
                    destination_array_order: 2,
                    value_expected: json!([[1]]),
                },
                TestCase {
                    value: json!([1, 2]),
                    destination_is_generic: false,
                    destination_array_order: 2,
                    value_expected: json!([[1, 2]]),
                },
                TestCase {
                    value: json!([[1, 2], [3, 4]]),
                    destination_is_generic: false,
                    destination_array_order: 2,
                    value_expected: json!([[1, 2], [3, 4]]),
                },
            ];

            for test_case in test_cases {
                // Setup
                let mut function = test_function();
                let destination = OutputConnection::new(
                    Source::default(),
                    0,
                    0,
                    0,
                    test_case.destination_array_order,
                    test_case.destination_is_generic,
                    String::default(),
                    #[cfg(feature = "debugger")]
                        String::default(),
                );

                // Test
                assert!(function.type_convert_and_send(&destination, &test_case.value));

                // Check
                assert_eq!(
                    test_case.value_expected,
                    function
                        .take_input_set()
                        .expect("Couldn't get input set")
                        .remove(0)
                );
            }
        }
    }
}

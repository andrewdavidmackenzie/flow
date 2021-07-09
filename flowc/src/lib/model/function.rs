use std::collections::HashMap;
use std::fmt;

use error_chain::bail;
use serde_derive::{Deserialize, Serialize};

use flowcore::input::InputInitializer;
use flowcore::output_connection::OutputConnection;
use flowcore::output_connection::Source::Output;

use crate::compiler::loader::Validate;
use crate::errors::*;
use crate::model::io::IOSet;
use crate::model::io::{IOType, IO};
use crate::model::name::HasName;
use crate::model::name::Name;
use crate::model::route::HasRoute;
use crate::model::route::Route;
use crate::model::route::SetIORoutes;
use crate::model::route::SetRoute;

/// Function defines a Function that implements some processing in the flow hierarchy
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct Function {
    /// `name` of the function
    #[serde(rename = "function")]
    name: Name,
    /// Is this an impure function that interacts with the runtime environment?
    #[serde(default)]
    impure: bool,
    /// String name of the file where the actual implementation should be read from
    implementation: String,
    /// The set of inputs this function has
    #[serde(default, rename = "input")]
    pub inputs: IOSet,
    /// The set of outputs this function generates when executed
    #[serde(default, rename = "output")]
    outputs: IOSet,

    /// As a function can be used multiple times in a single flow, the repeated instances must
    /// be referred to using an alias to disambiguate which instance is being referred to
    #[serde(skip_deserializing)]
    alias: Name,
    /// `source_url` is where this function definition was read from
    #[serde(skip_deserializing, default)]
    source_url: String, // can be a relative path with no scheme etc so can't be a Url
    /// the `route` in the flow hierarchy where this function is located
    #[serde(skip_deserializing)]
    route: Route,
    /// Is the function being used part of a library and where is it found
    #[serde(skip_deserializing)]
    lib_reference: Option<String>,

    #[serde(skip_deserializing)]
    output_connections: Vec<OutputConnection>,
    #[serde(skip_deserializing)]
    id: usize,
    #[serde(skip_deserializing)]
    flow_id: usize,
}

impl HasName for Function {
    fn name(&self) -> &Name {
        &self.name
    }
    fn alias(&self) -> &Name {
        &self.alias
    }
}

impl HasRoute for Function {
    fn route(&self) -> &Route {
        &self.route
    }
    fn route_mut(&mut self) -> &mut Route {
        &mut self.route
    }
}

impl Function {
    /// Create a new function - used mainly for testing as Functions are usually deserialized
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: Name,
        impure: bool,
        implementation: String,
        alias: Name,
        inputs: IOSet,
        outputs: IOSet,
        source_url: &str,
        route: Route,
        lib_reference: Option<String>,
        output_connections: Vec<OutputConnection>,
        id: usize,
        flow_id: usize,
    ) -> Self {
        Function {
            name,
            impure,
            implementation,
            alias,
            inputs,
            outputs,
            source_url: source_url.to_owned(),
            route,
            lib_reference,
            output_connections,
            id,
            flow_id,
        }
    }

    /// Set the id of this function
    pub fn set_id(&mut self, id: usize) {
        self.id = id;
    }

    /// Get the id of this function
    pub fn get_id(&self) -> usize {
        self.id
    }

    /// Set the id of the low this function is a part of  
    pub fn set_flow_id(&mut self, flow_id: usize) {
        self.flow_id = flow_id;
    }

    /// Get the id of the low this function is a part of  
    pub fn get_flow_id(&self) -> usize {
        self.flow_id
    }

    /// Return true if this function is impure or not
    pub fn is_impure(&self) -> bool {
        self.impure
    }

    /// Get a reference to the set of inputs of this function
    pub fn get_inputs(&self) -> &IOSet {
        &self.inputs
    }

    /// Get a mutable reference to the set of inputs of this function
    pub fn get_mut_inputs(&mut self) -> &mut IOSet {
        &mut self.inputs
    }

    /// Get a reference to the set of outputs this function generates
    pub fn get_outputs(&self) -> IOSet {
        self.outputs.clone()
    }

    /// Add a connection from this function to another
    pub fn add_output_route(&mut self, output_route: OutputConnection) {
        self.output_connections.push(output_route);
    }

    /// Get a reference to the set of output connections from this function to others
    pub fn get_output_connections(&self) -> &Vec<OutputConnection> {
        &self.output_connections
    }

    /// Get a reference to the implementation of this function
    pub fn get_implementation(&self) -> &str {
        &self.implementation
    }

    /// Set the implementation of this function
    pub fn set_implementation(&mut self, implementation: &str) {
        self.implementation = implementation.to_owned();
    }

    /// Get the source url where this function was defined
    pub fn get_source_url(&self) -> &str {
        &self.source_url
    }

    /// Set the source url where this function is defined
    pub fn set_source_url(&mut self, source: &str) {
        self.source_url = source.to_owned();
    }

    /// Set the alias of this function
    pub fn set_alias(&mut self, alias: &Name) {
        if alias.is_empty() {
            self.alias = self.name.clone();
        } else {
            self.alias = alias.clone();
        }
    }

    /// Set the initial values on the IOs in an IOSet using a set of Input Initializers
    pub fn set_initial_values(&mut self, initializers: &HashMap<String, InputInitializer>) {
        for initializer in initializers {
            // initializer.0 is io name, initializer.1 is the initial value to set it to
            for (index, input) in self.inputs.iter_mut().enumerate() {
                if *input.name() == Name::from(initializer.0)
                    || (initializer.0.as_str() == "default" && index == 0)
                {
                    input.set_initializer(&Some(initializer.1.clone()));
                }
            }
        }
    }

    /// Set the lib reference of this function
    pub fn set_lib_reference(&mut self, lib_reference: Option<String>) {
        self.lib_reference = lib_reference
    }

    /// Get the lib reference of this function
    pub fn get_lib_reference(&self) -> &Option<String> {
        &self.lib_reference
    }
}

impl Validate for Function {
    fn validate(&self) -> Result<()> {
        self.name.validate()?;

        let mut io_count = 0;

        for i in &self.inputs {
            io_count += 1;
            i.validate()?
        }

        for i in &self.outputs {
            io_count += 1;
            i.validate()?
        }

        // A function must have at least one valid input or output
        if io_count == 0 {
            bail!("A function must have at least one input or output");
        }

        Ok(())
    }
}

impl fmt::Display for Function {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "name: \t\t{}", self.name)?;
        writeln!(f, "alias: \t\t{}", self.alias)?;
        writeln!(f, "id: \t\t{}", self.id)?;
        writeln!(f, "flow_id: \t\t{}", self.flow_id)?;

        writeln!(f, "inputs:")?;
        for input in &self.inputs {
            writeln!(f, "\t{:#?}", input)?;
        }

        writeln!(f, "outputs:")?;
        for output in &self.outputs {
            writeln!(f, "\t{:#?}", output)?;
        }

        Ok(())
    }
}

impl Default for Function {
    fn default() -> Function {
        Function {
            name: Name::default(),
            impure: false,
            implementation: "".to_owned(),
            alias: Name::default(),
            inputs: vec![],
            outputs: vec![IO::new("Value", Route::default())],
            source_url: String::default(),
            route: Route::default(),
            lib_reference: None,
            output_connections: vec![OutputConnection::new(
                Output("".into()),
                0,
                0,
                0,
                0,
                false,
                String::default(),
                #[cfg(feature = "debugger")]
                String::default(),
            )],
            id: 0,
            flow_id: 0,
        }
    }
}

impl SetRoute for Function {
    fn set_routes_from_parent(&mut self, parent_route: &Route) {
        self.route = Route::from(format!("{}/{}", parent_route, self.alias));
        self.inputs
            .set_io_routes_from_parent(&self.route, IOType::FunctionIO);
        self.outputs
            .set_io_routes_from_parent(&self.route, IOType::FunctionIO);
    }
}

#[cfg(test)]
mod test {
    use flowcore::output_connection::OutputConnection;
    use flowcore::output_connection::Source::Output;

    use crate::compiler::loader::Validate;
    use crate::model::datatype::DataType;
    use crate::model::io::Find;
    use crate::model::name::HasName;
    use crate::model::name::Name;
    use crate::model::route::HasRoute;
    use crate::model::route::Route;
    use crate::model::route::SetRoute;

    use super::Function;

    #[test]
    fn function_with_no_io_not_valid() {
        let fun = Function {
            name: Name::from("test_function"),
            impure: false,
            implementation: "".to_owned(),
            alias: Name::from("test_function"),
            source_url: String::default(),
            inputs: vec![],  // No inputs!
            outputs: vec![], // No output!
            route: Route::default(),
            lib_reference: None,
            output_connections: vec![OutputConnection::new(
                Output("test_function".into()),
                0,
                0,
                0,
                0,
                false,
                String::default(),
                #[cfg(feature = "debugger")]
                String::default(),
            )],
            id: 0,
            flow_id: 0,
        };

        assert!(fun.validate().is_err());
    }

    #[test]
    fn deserialize_missing_name() {
        let function_str = "
        type = 'Value'
        ";

        let r_f: Result<Function, _> = toml::from_str(function_str);
        assert!(r_f.is_err());
    }

    #[test]
    fn deserialize_invalid() {
        let function_str = "
        name = 'test_function'
        ";

        let function: Result<Function, _> = toml::from_str(function_str);
        assert!(function.is_err());
    }

    #[test]
    fn deserialize_no_inputs_or_outputs() {
        let function_str = "
        function = 'test_function'
        implementation = 'test.rs'
        ";

        let function: Function =
            toml::from_str(function_str).expect("Couldn't read function from toml");
        assert!(function.validate().is_err());
    }

    #[test]
    fn deserialize_extra_field_fails() {
        let function_str = "
        function = 'test_function'
        implementation = 'test.rs'
        [[output]]
        foo = 'true'
        ";

        let function: Result<Function, _> = toml::from_str(function_str);
        assert!(function.is_err());
    }

    #[test]
    fn deserialize_default_output() {
        let function_str = "
        function = 'test_function'
        implementation = 'test.rs'
        [[output]]
        type = 'String'
        ";

        let function: Function =
            toml::from_str(function_str).expect("Couldn't read function from toml");
        function.validate().expect("Function did not validate");
        assert!(!function.outputs.is_empty());
        let output = &function.outputs[0];
        assert_eq!(*output.name(), Name::default());
        assert_eq!(output.datatype(), &DataType::from("String"));
    }

    #[test]
    fn deserialize_output_specified() {
        let function_str = "
        function = 'test_function'
        implementation = 'test.rs'

        [[output]]
        name = 'sub_output'
        type = 'String'
        ";

        let function: Function =
            toml::from_str(function_str).expect("Could not deserialize function from toml");
        function.validate().expect("Function does not validate");
        assert!(!function.outputs.is_empty());
        let output = &function.outputs[0];
        assert_eq!(*output.name(), Name::from("sub_output"));
        assert_eq!(output.datatype(), &DataType::from("String"));
    }

    #[test]
    fn deserialize_two_outputs_specified() {
        let function_str = "
        function = 'test_function'
        implementation = 'test.rs'

        [[output]]
        name = 'sub_output'
        type = 'String'
        [[output]]
        name = 'other_output'
        type = 'Number'
        ";

        let function: Function =
            toml::from_str(function_str).expect("Couldn't read function from toml");
        function.validate().expect("Function didn't validate");
        assert!(!function.outputs.is_empty());
        let outputs = function.outputs;
        assert_eq!(outputs.len(), 2);
        let output0 = &outputs[0];
        assert_eq!(*output0.name(), Name::from("sub_output"));
        assert_eq!(output0.datatype(), &DataType::from("String"));
        let output1 = &outputs[1];
        assert_eq!(*output1.name(), Name::from("other_output"));
        assert_eq!(output1.datatype(), &DataType::from("Number"));
    }

    #[test]
    fn set_routes() {
        let function_str = "
        function = 'test_function'
        implementation = 'test.rs'

        [[output]]
        name = 'sub_output'
        type = 'String'
        [[output]]
        name = 'other_output'
        type = 'Number'
        ";

        // Setup
        let mut function: Function =
            toml::from_str(function_str).expect("Couldn't read function from toml");
        function.alias = Name::from("test_alias");

        // Test
        function.set_routes_from_parent(&Route::from("/flow"));

        assert_eq!(function.route, Route::from("/flow/test_alias"));

        let output0 = &function.outputs[0];
        assert_eq!(*output0.route(), Route::from("/flow/test_alias/sub_output"));

        let output1 = &function.outputs[1];
        assert_eq!(
            *output1.route(),
            Route::from("/flow/test_alias/other_output")
        );
    }

    #[test]
    fn get_array_element_of_root_output() {
        // Create a function where the output is an Array of String
        let function_str = "
        function = 'test_function'
        implementation = 'test.rs'

        [[output]]
        type = 'Array/String'
        ";

        // Setup
        let mut function: Function =
            toml::from_str(function_str).expect("Couldn't read function from toml");
        function.alias = Name::from("test_alias");
        function.set_routes_from_parent(&Route::from("/flow"));

        // Test
        // Try and get the output using a route to a specific element of the output
        let output = function
            .outputs
            .find_by_route_and_set_initializer(&Route::from("/0"), &None)
            .expect("Expected to find an IO");
        assert_eq!(*output.name(), Name::default());
    }
}

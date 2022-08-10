use std::collections::HashMap;
use std::fmt;

use error_chain::bail;
use serde_derive::{Deserialize, Serialize};
use url::Url;

use crate::errors::*;
use crate::model::input::InputInitializer;
use crate::model::io::IOSet;
use crate::model::io::IOType;
use crate::model::name::HasName;
use crate::model::name::Name;
use crate::model::output_connection::OutputConnection;
use crate::model::route::HasRoute;
use crate::model::route::Route;
use crate::model::route::SetIORoutes;
use crate::model::route::SetRoute;
use crate::model::validation::Validate;

/// `FunctionDefinition` defines a Function (compile time) that implements some processing in the flow hierarchy
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct FunctionDefinition {
    /// `Name` of the function
    #[serde(rename = "function")]
    pub name: Name,
    /// Is this an impure function that interacts with the environment
    #[serde(default)]
    pub impure: bool,
    /// Name of the source file for the function implementation
    pub source: String,
    /// Name of any docs file associated with this Function
    #[serde(default)]
    pub docs: String,
    /// Type of build used to compile Function's implementation to WASM from source
    #[serde(default, rename = "type")]
    pub build_type: String,
    /// The set of inputs this function has
    #[serde(default, rename = "input")]
    pub inputs: IOSet,
    /// The set of outputs this function generates when executed
    #[serde(default, rename = "output")]
    pub outputs: IOSet,

    /// As a function can be used multiple times in a single flow, the repeated instances must
    /// be referred to using an alias to disambiguate which instance is being referred to
    #[serde(skip_deserializing)]
    pub alias: Name,
    /// `source_url` is where this function definition was read from
    #[serde(skip_deserializing, default = "FunctionDefinition::default_url")]
    pub source_url: Url,
    /// the `route` in the flow hierarchy where this function is located
    #[serde(skip_deserializing)]
    pub route: Route,
    /// Implementation is the relative path from the lib root to the compiled wasm implementation
    #[serde(skip_deserializing)]
    pub implementation: String,
    /// Is the function being used part of a library and where is it found
    #[serde(skip_deserializing)]
    pub lib_reference: Option<Url>,
    /// Is the function a context function and where is it found
    #[serde(skip_deserializing)]
    pub context_reference: Option<Url>,
    /// The output connections from this function to other processes (functions or flows)
    #[serde(skip_deserializing)]
    pub output_connections: Vec<OutputConnection>,
    /// A unique `id` assigned to the function as the flow is parsed hierarchically
    #[serde(skip_deserializing)]
    pub function_id: usize,
    /// the `id` of the `FlowDefinition` that this `FunctionDefinition` lies within in the hierarchy
    #[serde(skip_deserializing)]
    pub flow_id: usize,
}

impl Default for FunctionDefinition {
    fn default() -> Self {
        FunctionDefinition {
            name: Default::default(),
            impure: false,
            source: "".to_string(),
            docs: "".to_string(),
            build_type: "".to_string(),
            inputs: vec![],
            outputs: vec![],
            alias: Default::default(),
            source_url: FunctionDefinition::default_url(),
            route: Default::default(),
            implementation: "".to_string(),
            lib_reference: None,
            context_reference: None,
            output_connections: vec![],
            function_id: 0,
            flow_id: 0,
        }
    }
}

impl HasName for FunctionDefinition {
    fn name(&self) -> &Name {
        &self.name
    }
    fn alias(&self) -> &Name {
        &self.alias
    }
}

impl HasRoute for FunctionDefinition {
    fn route(&self) -> &Route {
        &self.route
    }
    fn route_mut(&mut self) -> &mut Route {
        &mut self.route
    }
}

impl FunctionDefinition {
    fn default_url() -> Url {
        Url::parse("file://").expect("Could not create default_url")
    }

    /// Create a new function - used mainly for testing as Functions are usually deserialized
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: Name,
        impure: bool,
        source: String,
        alias: Name,
        inputs: IOSet,
        outputs: IOSet,
        source_url: Url,
        route: Route,
        lib_reference: Option<Url>,
        context_reference: Option<Url>,
        output_connections: Vec<OutputConnection>,
        id: usize,
        flow_id: usize,
    ) -> Self {
        FunctionDefinition {
            name,
            impure,
            source,
            docs: String::default(),
            alias,
            inputs,
            outputs,
            source_url,
            route,
            implementation: String::default(),
            lib_reference,
            context_reference,
            output_connections,
            function_id: id,
            flow_id,
            build_type: String::default(),
        }
    }

    /// Configure a function with additional information after it is deserialized as part of a flow
    #[allow(clippy::too_many_arguments)]
    pub fn config(
        &mut self,
        original_url: &Url,
        source_url: &Url,
        parent_route: &Route,
        alias: &Name,
        flow_id: usize,
        reference: Option<Url>,
        initializations: &HashMap<String, InputInitializer>,
    ) -> Result<()> {
        self.set_flow_id(flow_id);
        self.set_alias(alias);
        self.set_source_url(source_url);
        if let Some(function_reference) = reference {
            match function_reference.scheme() {
                "context" => self.set_context_reference(Some(function_reference)),
                "lib" => self.set_lib_reference(Some(function_reference)),
                _ => {}
            }
        }
        self.set_routes_from_parent(parent_route);
        self.set_initial_values(initializations);
        self.check_impurity(original_url)?;
        self.validate()
    }

    /// Set the id of this function
    pub fn set_id(&mut self, id: usize) {
        self.function_id = id;
    }

    /// Get the id of this function
    pub fn get_id(&self) -> usize {
        self.function_id
    }

    /// Get the name of any associated docs file
    pub fn get_docs(&self) -> &str {
        &self.docs
    }

    // Set the id of the low this function is a part of
    fn set_flow_id(&mut self, flow_id: usize) {
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

    // A function can only be impure if it is provided by 'context'
    fn check_impurity(&self, url: &Url) -> Result<()> {
        if self.impure && url.scheme() != "context" {
            bail!("Only functions provided by 'context' can be impure ('{}')", url);
        }

        Ok(())
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
    pub fn add_output_connection(&mut self, output_route: OutputConnection) {
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

    /// Set the implementation location of this function
    pub fn set_implementation(&mut self, implementation: &str) {
        self.implementation = implementation.to_owned();
    }

    /// Set the source field of the function
    pub fn set_source(&mut self, source: &str) {
        self.source = source.to_owned()
    }

    /// Get the name of the source file relative to the function definition
    pub fn get_source(&self) -> &str {
        &self.source
    }

    /// Get the source url for the file where this function was defined
    pub fn get_source_url(&self) -> &Url {
        &self.source_url
    }

    // Set the source url where this function is defined
    fn set_source_url(&mut self, source: &Url) {
        self.source_url = source.clone();
    }

    // Set the alias of this function
    fn set_alias(&mut self, alias: &Name) {
        if alias.is_empty() {
            self.alias = self.name.clone();
        } else {
            self.alias = alias.clone();
        }
    }

    // Set the initial values on the IOs in an IOSet using a set of Input Initializers
    fn set_initial_values(&mut self, initializers: &HashMap<String, InputInitializer>) {
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

    /// Set the initial value on one of the IOs
    pub fn set_initial_value(&mut self, io_number: usize, initializer: &Option<InputInitializer>) -> Result<()> {
        self.inputs.get_mut(io_number).ok_or("No such input")?.set_initializer(initializer);
        Ok(())
    }

    // Set the lib reference of this function
    fn set_lib_reference(&mut self, lib_reference: Option<Url>) {
        self.lib_reference = lib_reference
    }

    /// Get the lib reference of this function
    pub fn get_lib_reference(&self) -> &Option<Url> {
        &self.lib_reference
    }

    // Set the context reference of this function
    fn set_context_reference(&mut self, context_reference: Option<Url>) {
        self.context_reference = context_reference
    }

    /// Get the context reference of this function
    pub fn get_context_reference(&self) -> &Option<Url> {
        &self.context_reference
    }

    /// Convert a FunctionDefinition filename into the name of the struct used to implement it
    /// by removing underscores and camel case each word
    /// Example ''duplicate_rows' -> 'DuplicateRows'
    pub fn camel_case(original: &str) -> String {
        // split into parts by '_' and Uppercase the first character of the (ASCII) Struct name
        let words: Vec<String> = original
            .split('_')
            .map(|w| format!("{}{}", (w[..1].to_string()).to_uppercase(), &w[1..]))
            .collect();
        // recombine
        words.join("")
    }
}

impl Validate for FunctionDefinition {
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

impl fmt::Display for FunctionDefinition {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "name: \t\t{}", self.name)?;
        writeln!(f, "alias: \t\t{}", self.alias)?;
        writeln!(f, "id: \t\t{}", self.function_id)?;
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

impl SetRoute for FunctionDefinition {
    fn set_routes_from_parent(&mut self, parent_route: &Route) {
        self.route = Route::from(format!("{}/{}", parent_route, self.alias));
        self.inputs
            .set_io_routes_from_parent(&self.route, IOType::FunctionInput);
        self.outputs
            .set_io_routes_from_parent(&self.route, IOType::FunctionOutput);
    }
}

#[cfg(test)]
mod test {
    use url::Url;

    use crate::deserializers::deserializer::get_deserializer;
    use crate::errors::*;
    use crate::model::datatype::{DataType, NUMBER_TYPE, STRING_TYPE};
    use crate::model::name::HasName;
    use crate::model::name::Name;
    use crate::model::output_connection::OutputConnection;
    use crate::model::output_connection::Source::Output;
    use crate::model::route::HasRoute;
    use crate::model::route::Route;
    use crate::model::route::SetRoute;
    use crate::model::validation::Validate;

    use super::FunctionDefinition;

    #[test]
    fn function_with_no_io_not_valid() {
        let fun = FunctionDefinition {
            name: Name::from("test_function"),
            alias: Name::from("test_function"),
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
            ..Default::default()
        };

        assert!(fun.validate().is_err());
    }

    fn toml_from_str(content: &str) -> Result<FunctionDefinition> {
        let url = Url::parse("file:///fake.toml").expect("Could not parse URL");
        let deserializer = get_deserializer::<FunctionDefinition>(&url).expect("Could not get deserializer");
        deserializer.deserialize(content, Some(&url))
    }

    #[test]
    fn deserialize_missing_name() {
        let function_str = "
        type = 'object'
        ";

        let r_f: Result<FunctionDefinition> = toml_from_str(function_str);
        assert!(r_f.is_err());
    }

    #[test]
    fn deserialize_invalid() {
        let function_str = "
        name = 'test_function'
        ";

        let function: Result<FunctionDefinition> = toml_from_str(function_str);
        assert!(function.is_err());
    }

    #[test]
    fn deserialize_no_inputs_or_outputs() {
        let function_str = "
        function = 'test_function'
        source = 'test.rs'
        ";

        let function: FunctionDefinition =
            toml_from_str(function_str).expect("Couldn't read function from toml");
        assert!(function.validate().is_err());
    }

    #[test]
    fn deserialize_extra_field_fails() {
        let function_str = "
        function = 'test_function'
        source = 'test.rs'
        [[output]]
        foo = 'true'
        ";

        let function: Result<FunctionDefinition> = toml_from_str(function_str);
        assert!(function.is_err());
    }

    #[test]
    fn impure_not_allowed() {
        let function_str = "
        function = 'disallowed_impure'
        source = 'disallowed_impure.rs'
        docs = 'disallowed_impure.md'
        type = 'rust'
        impure = true

        [[input]]
        name = 'left'
        type = 'number'
        ";

        let function = toml_from_str(function_str)
            .expect("Couldn't load function from toml");
        assert!(function.check_impurity(function.get_source_url()).is_err());
    }

    #[test]
    fn deserialize_default_output() {
        let function_str = "
        function = 'test_function'
        source = 'test.rs'
        [[output]]
        type = 'string'
        ";

        let function: FunctionDefinition =
            toml_from_str(function_str).expect("Couldn't read function from toml");
        function.validate().expect("Function did not validate");
        assert!(!function.outputs.is_empty());
        let output = &function.outputs[0];
        assert_eq!(*output.name(), Name::default());
        assert_eq!(output.datatypes().len(), 1);
        assert_eq!(output.datatypes()[0], DataType::from(STRING_TYPE));
    }

    #[test]
    fn deserialize_output_specified() {
        let function_str = "
        function = 'test_function'
        source = 'test.rs'

        [[output]]
        name = 'sub_output'
        type = 'string'
        ";

        let function: FunctionDefinition =
            toml_from_str(function_str).expect("Could not deserialize function from toml");
        function.validate().expect("Function does not validate");
        assert!(!function.outputs.is_empty());
        let output = &function.outputs[0];
        assert_eq!(*output.name(), Name::from("sub_output"));
        assert_eq!(output.datatypes().len(), 1);
        assert_eq!(output.datatypes()[0], DataType::from(STRING_TYPE));
    }

    #[test]
    fn deserialize_two_outputs_specified() {
        let function_str = "
        function = 'test_function'
        source = 'test.rs'

        [[output]]
        name = 'sub_output'
        type = 'string'
        [[output]]
        name = 'other_output'
        type = 'number'
        ";

        let function: FunctionDefinition =
            toml_from_str(function_str).expect("Couldn't read function from toml");
        function.validate().expect("Function didn't validate");
        assert!(!function.outputs.is_empty());
        let outputs = function.outputs;
        assert_eq!(outputs.len(), 2);
        let output0 = &outputs[0];
        assert_eq!(*output0.name(), Name::from("sub_output"));
        assert_eq!(output0.datatypes().len(), 1);
        assert_eq!(output0.datatypes()[0], DataType::from(STRING_TYPE));
        let output1 = &outputs[1];
        assert_eq!(*output1.name(), Name::from("other_output"));
        assert_eq!(output1.datatypes().len(), 1);
        assert_eq!(output1.datatypes()[0], DataType::from(NUMBER_TYPE));
    }

    #[test]
    fn set_routes() {
        let function_str = "
        function = 'test_function'
        source = 'test.rs'

        [[output]]
        name = 'sub_output'
        type = 'string'
        [[output]]
        name = 'other_output'
        type = 'number'
        ";

        // Setup
        let mut function: FunctionDefinition =
            toml_from_str(function_str).expect("Couldn't read function from toml");
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
}

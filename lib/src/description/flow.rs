use description::name::Name;
use description::name::Named;
use description::connection::Connection;
use description::io::IO;
use description::value::Value;
use loader::loader::Validate;
use description::function::FunctionReference;

use std::fmt;
use std::path::PathBuf;

#[derive(Default, Deserialize, Debug)]
pub struct FlowReference {
    #[serde(rename = "name")]
    pub alias: Name,
    pub source: String,
    #[serde(skip_deserializing)]
    pub route: String,
    #[serde(skip_deserializing)]
    pub flow: Flow
}

// TODO figure out how to have this derived automatically for types needing it
impl Named for FlowReference {
    fn name(&self) -> &str {
        &self.alias[..]
    }
}

impl Validate for FlowReference {
    fn validate(&self) -> Result<(), String> {
        self.alias.validate()
        // Pretty much anything is a valid PathBuf - so not sure how to validate source...
    }
}

impl fmt::Display for FlowReference {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\t\t\t\talias: {}\n\t\t\t\t\tsource: {}\n\t\t\t\t\troute: {}\n",
               self.alias, self.source, self.route)
    }
}

#[derive(Default, Deserialize, Debug)]
pub struct Flow {
    #[serde(skip_deserializing)]
    pub source: PathBuf,
    pub name: Name,
    #[serde(skip_deserializing)]
    pub route: String,

    pub flow: Option<Vec<FlowReference>>,
    pub function: Option<Vec<FunctionReference>>,

    pub value: Option<Vec<Value>>,

    pub input: Option<Vec<IO>>,
    pub output: Option<Vec<IO>>,
    pub connection: Option<Vec<Connection>>,
}

// TODO figure out how to have this derived automatically for types needing it
impl Named for Flow {
    fn name(&self) -> &str {
        &self.name[..]
    }
}

impl Validate for Flow {
    // check the correctness of all the fields in this flow, prior to loading sub-elements
    fn validate(&self) -> Result<(), String> {
        self.name.validate()?;

        if let Some(ref flows_refs) = self.flow {
            for flow_ref in flows_refs {
                flow_ref.validate()?;
            }
        }

        if let Some(ref function_refs) = self.function {
            for function_ref in function_refs {
                function_ref.validate()?;
            }
        }

        if let Some(ref inputs) = self.input {
            for input in inputs {
                input.validate()?;
            }
        }

        if let Some(ref outputs) = self.output {
            for output in outputs {
                output.validate()?;
            }
        }

        if let Some(ref values) = self.value {
            for value in values {
                value.validate()?;
            }
        }

        Ok(())
    }
}

impl fmt::Display for Flow {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\tname: \t\t\t{}\n\tsource: \t\t{}\n\troute: \t\t\t{}\n",
               self.name, self.source.display(), self.route).unwrap();

        write!(f, "\tvalues:\n").unwrap();
        if let Some(ref values) = self.value {
            for value in values {
                write!(f, "\t\t\t\t{}\n", value).unwrap();
            }
        }

        write!(f, "\tinputs:\n").unwrap();
        if let Some(ref inputs) = self.input {
            for input in inputs {
                write!(f, "\t\t\t\t{}\n", input).unwrap();
            }
        }

        write!(f, "\touputs:\n").unwrap();
        if let Some(ref outputs) = self.output {
            for output in outputs {
                write!(f, "\t\t\t\t{}\n", output).unwrap();
            }
        }

        write!(f, "\tsubflows:\n").unwrap();
        if let Some(ref flow_refs) = self.flow {
            for flow_ref in flow_refs {
                write!(f, "\t{}\n", flow_ref).unwrap();
            }
        }

        write!(f, "\tfunctions: \t\n").unwrap();
        if let Some(ref function_refs) = self.function {
            for function_ref in function_refs {
                write!(f, "\t{}", function_ref).unwrap();
                write!(f, "\t{}", function_ref.function).unwrap();
            }
        }

        write!(f, "\tconnections: \t\n").unwrap();
        if let Some(ref connections) = self.connection {
            for connection in connections {
                write!(f, "\t\t\t\t\t{} --> {}\n", connection.from, connection.to).unwrap();
            }
        }

        Ok(())
    }
}

impl Flow {
    // TODO Better to write this as a function/trait on other struct and test it
    fn name_in_collection<N: Named>(collection: &Option<Vec<N>>, element_name: &str) -> Result<(), String> {
        if let &Some(ref elements) = collection {
            for element in elements {
                if element.name() == element_name {
                    return Ok(());
                }
            }
        }
        Err(format!("Name '{}' was not found", element_name))
    }

    /*
        Check that the name of an io is valid and it exists in the flow
            Connection to/from Formats:
            "flow/this/out"
            "flow/hello/out"
            "function/print/stdout"
            "value/message"
     */
    fn io_name_valid(&self, io_name: &Name) -> Result<(), String> {
        let segments: Vec<&str> = io_name.split('/').collect();
        match segments.len() {
            2 => {
                match (segments[0], segments[1]) {
                    ("value", value_name) => Flow::name_in_collection(&self.value, value_name),
                    ("input", input) => Flow::name_in_collection(&self.input, input),
                    ("output", output) => Flow::name_in_collection(&self.output, output),
                    _ => Err(format!("Invalid name '{}' used in connection", io_name))
                }
            }
            3 => {
                match (segments[0], segments[1], segments[2]) {
                    ("flow", flow_name, _) => Flow::name_in_collection(&self.flow, flow_name),
                    ("function", function_name, _) => Flow::name_in_collection(&self.function, function_name),
                    _ => Err(format!("Invalid name '{}' used in connection", io_name))
                }
            }
            _ => Err(format!("Invalid name '{}' used in connection", io_name))
        }
    }

    /*
        This is run after references have been loaded, so the full io name can be checked
        in connections.
    */
    pub fn check_connections(&self) -> Result<(), String> {
        if let Some(ref connections) = self.connection {
            for connection in connections {
                connection.validate()?;
                self.io_name_valid(&connection.from)?;
                self.io_name_valid(&connection.to)?;
            }
        }
        Ok(())
    }
}
use description::name::Name;
use description::name::Named;
use description::connection::Connection;
use description::io::IO;
use description::function::Function;
use description::value::Value;
use loader::loader::Validate;
use loader::loader::Reference;

use std::fmt;
use std::path::PathBuf;

#[derive(Deserialize)]
pub struct Flow {
    #[serde(skip_deserializing)]
    pub source: PathBuf,
    pub name: Name,

    pub flow: Option<Vec<Reference>>,
    pub function: Option<Vec<Reference>>,

    pub value: Option<Vec<Value>>,

    pub input: Option<Vec<IO>>,
    pub output: Option<Vec<IO>>,
    pub connection: Option<Vec<Connection>>,

    #[serde(skip_deserializing)]
    pub flows: Vec<Flow>,
    #[serde(skip_deserializing)]
    pub functions: Vec<Function>
}

pub enum Direction {
    Input,
    Output
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

        if let Some(ref connections) = self.connection {
            for connection in connections {
                connection.validate()?;
                Flow::io_name_valid(&self, &connection.from)?;
                Flow::io_name_valid(&self, &connection.to)?;
            }
        }

        Ok(())
    }
}

impl Flow {
    // now that all is loaded, check all is OK
    pub fn verify(&self) -> Result<(), String> {
        // Need the connections hooked up by name to the actual IOs

        // TODO Check the connections and connect them up with refs?
        // pub connection: Option<Vec<Connection>>,
        // check connection directions and types
        // Check connections referring to IOs of this flow match those IOs
        // check connections referring to values of this flow match those values
        // Internal connection consistency io names exist, directions match, types match

        Ok(())
    }

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

    // Check that the name of an io is valid and it exists in the flow
    // Connection to/from Formats:
    // "flow/this/out"
    // "flow/hello/out"
    // "function/print/stdout"
    // "value/message"
    fn io_name_valid(&self, io_name: &Name) -> Result<(), String> {
        let segments: Vec<&str> = io_name.split('/').collect();
        match segments.len() {
            2 => {
                match (segments[0], segments[1]) {
                    ("value", value_name) => Flow::name_in_collection(&self.value, value_name),
                    _ => Err(format!("Invalid io name '{}' used in connection", io_name))
                }
            }
            3 => {
                match (segments[0], segments[1], segments[2]) {
                    ("flow", "this", io) => {
                        eprintln!("3 segments flow, this, {}", io);
                        if let Err(_) = Flow::name_in_collection(&self.input, io) {
                            return Flow::name_in_collection(&self.output, io)
                        }
                        Ok(())
                    }
                    ("flow", flow_name, _) => {
                        eprintln!("3 segments flow, {}, _", flow_name);
                        Flow::name_in_collection(&self.flow, flow_name)
                    },
                    ("function", function_name, _) => {
                        eprintln!("3 segments function, {}, _", function_name);
                        Flow::name_in_collection(&self.function, function_name)
                    },
                    _ => Err(format!("Invalid io name '{}' used in connection", io_name))
                }
            }
            _ => Err(format!("Invalid io name '{}' used in connection", io_name))
        }
    }
}

impl fmt::Display for Flow {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\nFlow:\n\tname: {}\n\tReferences: {:?}\n\tvalue: {:?}\n\tinputs: {:?}\n\toutputs: {:?}\n\tFunctionRefs: {:?}\n\tconnection: {:?}",
               self.name, self.flow, self.value, self.input, self.output, self.function, self.connection)
    }
}
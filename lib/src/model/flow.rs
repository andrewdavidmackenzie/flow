use model::name::Name;
use model::name::HasName;
use model::name::HasRoute;
use model::connection::Connection;
use model::io::IO;
use model::value::Value;
use model::flow_reference::FlowReference;
use loader::loader::Validate;
use model::function_reference::FunctionReference;

use std::fmt;
use std::path::PathBuf;

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

impl HasName for Flow {
    fn name(&self) -> &str {
        &self.name[..]
    }
}

impl HasRoute for Flow {
    fn route(&self) -> &str {
        &self.route[..]
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

        // TODO dry this all up now it works.

        write!(f, "\tvalues:\n").unwrap();
        if let Some(ref values) = self.value {
            for value in values {
                write!(f, "\t\t\t\t{}\n", value).unwrap();
            }
        }

        write!(f, "\tinputs:\n").unwrap();
        if let Some(ref inputs) = self.input {
            for input in inputs {
                write!(f, "\t\t\t\t\t{}\n", input).unwrap();
            }
        }

        write!(f, "\touputs:\n").unwrap();
        if let Some(ref outputs) = self.output {
            for output in outputs {
                write!(f, "\t\t\t\t\t{}\n", output).unwrap();
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
                write!(f, "\t\t\t\t\t{}\n", connection).unwrap();
            }
        }

        Ok(())
    }
}

impl Flow {
    fn find_route_by_name<E: HasName + HasRoute>(collection: &Option<Vec<E>>, element_name: &str)
                                                 -> Result<String, String> {
        if let &Some(ref elements) = collection {
            for element in elements {
                if element.name() == element_name {
                    return Ok(format!("{}", element.route()));
                }
            }
        }
        Err(format!("No element with name '{}' was found", element_name))
    }

    /*
        Check that the name of an io is valid and it exists in the flow
            Connection to/from Formats:
            "flow/this/out"
            "flow/hello/out"
            "function/print/stdout"
            "value/message"
     */
    fn io_name_exists(&self, io_name: &Name) -> Result<String, String> {
        let segments: Vec<&str> = io_name.split('/').collect();
        match segments.len() {
            2 => {
                match (segments[0], segments[1]) {
                    ("value", value_name) => Flow::find_route_by_name(&self.value, value_name),
                    ("input", input) => Flow::find_route_by_name(&self.input, input),
                    ("output", output) => Flow::find_route_by_name(&self.output, output),
                    _ => Err(format!("Invalid name '{}' used in connection", io_name))
                }
            }
            3 => {
                match (segments[0], segments[1], segments[2]) {
                    ("flow", flow_name, _) => Flow::find_route_by_name(&self.flow, flow_name),
                    ("function", function_name, _) => Flow::find_route_by_name(&self.function, function_name),
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
                self.io_name_exists(&connection.from)?;
                self.io_name_exists(&connection.to)?;
            }
        }
        Ok(())
    }

    /*
    Change IO names to the hierarchical format, using the internal name of the thing referenced
    // TODO create an IOSet type and move this in there
    */
    pub fn normalize_io_names(&mut self) {
        if let &mut Some(ref mut ios) = &mut self.input {
            for ref mut io in ios {
                io.route = format!("{}/{}", self.route, io.name);
            }
        }
        if let &mut Some(ref mut ios) = &mut self.output {
            for ref mut io in ios {
                io.route = format!("{}/{}", self.route, io.name);
            }
        }
    }

    /*
        Change the names of connections to be routes to the alias used in this flow
    */
    // TODO this is a mess and needs a total re-think. Also, it's not checking connection sense...
    pub fn normalize_connection_names(&mut self) {
        if let &mut Some(ref mut connections) = &mut self.connection {
            for ref mut connection in connections {

                let segments: Vec<&str> = connection.from.split('/').collect();
                match segments.len() {
                    2 => match (segments[0], segments[1]) {
                        ("value", value_name) => {
                            if let Ok(_) = Flow::find_route_by_name(&self.value, value_name) {
                                connection.from_route = format!("{}/{}", self.route, value_name);
                            }
                        },
                        ("input", input_name) => {
                            if let Ok(_) = Flow::find_route_by_name(&self.input, input_name) {
                                connection.from_route = format!("{}/{}", self.route, input_name);
                            }
                        },
                        _ => println!("Invalid name '{}' used in connection", connection.from)
                    },
                    3 => match (segments[0], segments[1], segments[2]) {
                        ("flow", flow_name, output_name) => {
                            if let Ok(flow_route) = Flow::find_route_by_name(&self.flow, flow_name) {
                                connection.from_route = format!("{}/{}", flow_route, output_name);
                            }
                        },
                        ("function", function_name, output_name) => {
                            if let Ok(function_route) = Flow::find_route_by_name(&self.function, function_name) {
                                connection.from_route = format!("{}/{}", function_route, output_name);
                            }
                        },
                        _ => println!("Invalid name '{}' used in connection", connection.from)
                    },
                    _ => println!("Invalid name '{}' used in connection", connection.from)
                }

                let segments: Vec<&str> = connection.to.split('/').collect();
                match segments.len() {
                    2 => match (segments[0], segments[1]) {
                        ("value", value_name) => {
                            if let Ok(_) = Flow::find_route_by_name(&self.value, value_name) {
                                connection.to_route = format!("{}/{}", self.route, value_name);
                            }
                        },
                        ("output", output_name) => {
                            if let Ok(_) = Flow::find_route_by_name(&self.output, output_name) {
                                connection.to_route = format!("{}/{}", self.route, output_name);
                            }
                        },
                        _ => println!("Invalid name '{}' used in connection", connection.to)
                    },
                    3 => match (segments[0], segments[1], segments[2]) {
                        ("flow", flow_name, output_name) => {
                            if let Ok(flow_route) = Flow::find_route_by_name(&self.flow, flow_name) {
                                connection.to_route = format!("{}/{}", flow_route, output_name);
                            }
                        },
                        ("function", function_name, output_name) => {
                            if let Ok(function_route) = Flow::find_route_by_name(&self.function, function_name) {
                                connection.to_route = format!("{}/{}", function_route, output_name);
                            }
                        },
                        _ => println!("Invalid name '{}' used in connection", connection.to)
                    },
                    _ => println!("Invalid name '{}' used in connection", connection.to)
                }
            }
        }
    }
}
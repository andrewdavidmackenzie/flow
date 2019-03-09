use model::name::Name;
use model::name::HasName;
use model::connection::Connection;
use model::io::IO;
use model::io::IOSet;
use model::value::Value;
use model::process_reference::ProcessReference;
use model::route::Route;
use model::route::HasRoute;
use model::route::SetRoute;
use model::io::Find;
use loader::loader::Validate;
use model::connection::Direction;
use model::runnable::Runnable;
use model::process::Process::FlowProcess;
use model::process::Process::FunctionProcess;
use serde_json::Value as JsonValue;
use std::fmt;
use std::collections::HashMap;

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct Flow {
    #[serde(rename = "flow")]
    pub name: Name,
    #[serde(rename = "input")]
    pub inputs: IOSet,
    #[serde(rename = "output")]
    pub outputs: IOSet,
    #[serde(rename = "process")]
    pub process_refs: Option<Vec<ProcessReference>>,
    #[serde(rename = "value")]
    pub values: Option<Vec<Value>>,
    #[serde(rename = "connection")]
    pub connections: Option<Vec<Connection>>,

    #[serde(default = "Flow::default_version")]
    pub version: String,
    #[serde(default = "Flow::default_author")]
    pub author_name: String,
    #[serde(default = "Flow::default_email")]
    pub author_email: String,

    #[serde(skip_deserializing)]
    pub alias: Name,
    #[serde(skip_deserializing, default = "Flow::default_url")]
    pub source_url: String,
    #[serde(skip_deserializing)]
    pub route: Route,
    #[serde(skip_deserializing)]
    pub lib_references: Vec<String>,
    #[serde(skip_deserializing)]
    pub initializations: Option<HashMap<String, JsonValue>>
}

impl Validate for Flow {
    // check the correctness of all the fields in this flow, prior to loading sub-elements
    fn validate(&self) -> Result<(), String> {
        if let Some(ref process_refs) = self.process_refs {
            for process_ref in process_refs {
                process_ref.validate()?;
            }
        }

        if let Some(ref inputs) = self.inputs {
            for input in inputs {
                input.validate()?;
            }
        }

        if let Some(ref outputs) = self.outputs {
            for output in outputs {
                output.validate()?;
            }
        }

        if let Some(ref values) = self.values {
            for value in values {
                value.validate()?;
            }
        }

        if let Some(ref connections) = self.connections {
            for connection in connections {
                connection.validate()?;
            }
        }

        Ok(())
    }
}

impl fmt::Display for Flow {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\tname: \t\t\t{}\n\talias: \t\t\t{}\n\tsource_url: \t{}\n\troute: \t\t\t{}\n",
               self.name, self.alias, self.source_url, self.route).unwrap();

        write!(f, "\tvalues:\n").unwrap();
        if let Some(ref values) = self.values {
            for value in values {
                write!(f, "\t\t\t\t{}\n", value).unwrap();
            }
        }

        write!(f, "\tinputs:\n").unwrap();
        if let Some(ref inputs) = self.inputs {
            for input in inputs {
                write!(f, "\t\t\t\t\t{:#?}\n", input).unwrap();
            }
        }

        write!(f, "\touputs:\n").unwrap();
        if let Some(ref outputs) = self.outputs {
            for output in outputs {
                write!(f, "\t\t\t\t\t{:#?}\n", output).unwrap();
            }
        }

        write!(f, "\tprocesses:\n").unwrap();
        if let Some(ref process_refs) = self.process_refs {
            for flow_ref in process_refs {
                write!(f, "\t{}\n", flow_ref).unwrap();
            }
        }

        write!(f, "\tconnections: \t\n").unwrap();
        if let Some(ref connections) = self.connections {
            for connection in connections {
                write!(f, "\t\t\t\t\t{}\n", connection).unwrap();
            }
        }

        Ok(())
    }
}

impl Default for Flow {
    fn default() -> Flow {
        Flow {
            name: "".to_string(),
            alias: "".to_string(),
            source_url: Flow::default_url(),
            route: "".to_string(),
            process_refs: None,
            values: None,
            inputs: None,
            outputs: None,
            connections: None,
            lib_references: vec!(),
            version: Flow::default_version(),
            author_name: Flow::default_author(),
            author_email: Flow::default_email(),
            initializations: None
        }
    }
}

impl HasRoute for Flow {
    fn route(&self) -> &Route {
        &self.route
    }
}

impl SetRoute for Flow {
    fn set_routes_from_parent(&mut self, parent_route: &Route, flow_io: bool) {
        self.route = format!("{}/{}", parent_route, self.alias);
        self.inputs.set_routes_from_parent(&self.route, flow_io);
        self.outputs.set_routes_from_parent(&self.route, flow_io);
    }
}

impl Flow {
    fn default_url() -> String {
        "file:///".to_string()
    }

    pub fn default_version() -> String {
        "0.0.0".to_string()
    }

    pub fn default_author() -> String {
        "unknown".to_string()
    }

    pub fn default_email() -> String {
        "unknown@unknown.com".to_string()
    }

    fn get_io_subprocess(&self, subprocess_alias: &str, direction: Direction, route: &Route) -> Result<IO, String> {
        if let Some(ref process_refs) = self.process_refs {
            for process_ref in process_refs {
                debug!("\tLooking in process_ref with alias = '{}'", process_ref.alias);
                match process_ref.process {
                    FlowProcess(ref flow) => {
                        if process_ref.name() == subprocess_alias {
                            debug!("\tFlow sub-process with matching name found, name = '{}'", process_ref.alias);
                            return match direction {
                                Direction::TO => flow.inputs.find_by_name(route),
                                Direction::FROM => flow.outputs.find_by_name(route)
                            };
                        }
                    },
                    FunctionProcess(ref function) => {
                        if process_ref.name() == subprocess_alias {
                            return match direction {
                                Direction::TO => function.get_inputs().find_by_route(route),
                                Direction::FROM => function.get_outputs().find_by_route(route)
                            };
                        }
                    },
                }
            }
            return Err(format!("Could not find sub-process named '{}'", subprocess_alias));
        }

        return Err("No sub-process present".to_string());
    }

    /*
        Find an IO of a value using the direction (TO/FROM) and the route to the IO
    */
    fn get_io_from_value(&self, value_name: &str, direction: Direction, route: &Route) -> Result<IO, String> {
        if let Some(values) = &self.values {
            for value in values {
                if value.name() == value_name {
                    return match direction {
                        Direction::TO => value.get_input(),
                        Direction::FROM => value.get_outputs().find_by_route(route)
                    };
                }
            }
            return Err(format!("Could not find value named '{}'", value_name));
        }

        return Err("No values present".to_string());
    }

    // TODO consider finding the object first using it's type and name (flow, subflow, value, function)
    // Then from the object find the IO (by name or route, probably route) in common code, maybe using IOSet directly?
    pub fn get_route_and_type(&mut self, direction: Direction, conn_descriptor: &str) -> Result<IO, String> {
        let mut segments: Vec<&str> = conn_descriptor.split('/').collect();
        let object_type = segments.remove(0); // first part is type of object
        let object_name = &Name::from(segments.remove(0)); // second part is the name of it
        let route = segments.join("/");       // the rest is a sub-route

        debug!("Looking for connection {:?} '{}' called '{}' with route '{}'", direction, object_type, object_name, route);

        match (&direction, object_type) {
            (&Direction::TO, "output") => self.outputs.find_by_name(object_name), // an output from this flow
            (&Direction::FROM, "input") => self.inputs.find_by_name(object_name), // an input to this flow
            (_, "process") => self.get_io_subprocess(object_name, direction, &route), // input or output of a sub-process
            (_, "value") => self.get_io_from_value(object_name, direction, &route), // input or output of a contained value
            _ => Err(format!("Unknown type of object '{}' used in IO descriptor '{}'", object_type, conn_descriptor))
        }
    }
}
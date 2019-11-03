use std::fmt;
use std::mem::replace;

use flowrlib::input::InputInitializer;

use crate::compiler::loader::Validate;
use crate::errors::*;
use crate::model::connection::Connection;
use crate::model::connection::Direction;
use crate::model::connection::Direction::FROM;
use crate::model::connection::Direction::TO;
use crate::model::io::{IO, IOType};
use crate::model::io::Find;
use crate::model::io::IOSet;
use crate::model::name::HasName;
use crate::model::name::Name;
use crate::model::process::Process::FlowProcess;
use crate::model::process::Process::FunctionProcess;
use crate::model::process_reference::ProcessReference;
use crate::model::route::HasRoute;
use crate::model::route::Route;
use crate::model::route::SetIORoutes;
use crate::model::route::SetRoute;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct Flow {
    #[serde(rename = "flow")]
    pub name: Name,
    #[serde(rename = "input")]
    inputs: IOSet,
    #[serde(rename = "output")]
    outputs: IOSet,
    #[serde(rename = "process")]
    pub process_refs: Option<Vec<ProcessReference>>,
    #[serde(rename = "connection")]
    pub connections: Option<Vec<Connection>>,

    #[serde(default = "Flow::default_description")]
    pub description: String,
    #[serde(default = "Flow::default_version")]
    pub version: String,
    #[serde(default = "Flow::default_author")]
    pub author_name: String,
    #[serde(default = "Flow::default_email")]
    pub author_email: String,

    #[serde(skip)]
    pub alias: Name,
    #[serde(skip, default = "Flow::default_url")]
    pub source_url: String,
    #[serde(skip)]
    pub route: Route,
    #[serde(skip)]
    pub lib_references: Vec<String>,
}

impl Validate for Flow {
    // check the correctness of all the fields in this flow, prior to loading sub-elements
    fn validate(&self) -> Result<()> {
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
            name: Name::default(),
            alias: Name::default(),
            source_url: Flow::default_url(),
            route: Route::default(),
            process_refs: None,
            inputs: None,
            outputs: None,
            connections: None,
            lib_references: vec!(),
            description: Flow::default_description(),
            version: Flow::default_version(),
            author_name: Flow::default_author(),
            author_email: Flow::default_email(),
        }
    }
}

impl HasName for Flow {
    fn name(&self) -> &Name {
        &self.name
    }

    fn alias(&self) -> &Name {
        &self.alias
    }
}

impl HasRoute for Flow {
    fn route(&self) -> &Route {
        &self.route
    }
}

impl SetRoute for Flow {
    fn set_routes_from_parent(&mut self, parent_route: &Route) {
        self.route = Route::from(format!("{}/{}", parent_route, self.alias));
        self.inputs.set_io_routes_from_parent(&self.route, IOType::FlowInput);
        self.outputs.set_io_routes_from_parent(&self.route, IOType::FlowOutput);
    }
}

impl Flow {
    fn default_url() -> String {
        "file:///".to_string()
    }

    pub fn default_description() -> String {
        "".into()
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

    pub fn inputs(&self) -> &IOSet {
        &self.inputs
    }

    pub fn inputs_mut(&mut self) -> &mut IOSet {
        &mut self.inputs
    }

    pub fn outputs(&self) -> &IOSet {
        &self.outputs
    }

    // TODO create a trait HasInputs and HasOutputs and implement it for function and flow
    // and process so this below can avoid the match
    fn get_io_subprocess(&mut self, subprocess_alias: &Name, direction: Direction, route: &Route,
                         initial_value: &Option<InputInitializer>) -> Result<IO> {
        if let Some(ref mut process_refs) = self.process_refs {
            for process_ref in process_refs {
                debug!("\tLooking in process_ref with alias = '{}'", process_ref.alias);
                if *subprocess_alias == process_ref.alias().clone() {
                    match process_ref.process {
                        FlowProcess(ref mut sub_flow) => {
                            debug!("\tFlow sub-process with matching name found, name = '{}'", process_ref.alias);
                            let io_name = Name::from(route);
                            return match direction {
                                Direction::TO => sub_flow.inputs.find_by_name(&io_name, initial_value),
                                Direction::FROM => sub_flow.outputs.find_by_name(&io_name, &None)
                            };
                        }
                        FunctionProcess(ref mut function) => {
                            return match direction {
                                Direction::TO => function.inputs.find_by_route(route, initial_value),
                                Direction::FROM => function.get_outputs().find_by_route(route, &None)
                            };
                        }
                    }
                }
            }
            bail!("Could not find sub-process named '{}'", subprocess_alias);
        }

        bail!("No sub-processes present");
    }

    // TODO consider finding the object first using it's type and name (flow, subflow, value, function)
    // Then from the object find the IO (by name or route, probably route) in common code, maybe using IOSet directly?
    pub fn get_route_and_type(&mut self, direction: Direction, conn_descriptor: &str,
                              initial_value: &Option<InputInitializer>) -> Result<IO> {
        let mut segments: Vec<&str> = conn_descriptor.split('/').collect();
        if segments.len() < 2 {
            bail!("Invalid route '{}'", conn_descriptor);
        }

        let object_type = segments.remove(0); // first part is type of object
        let object_name = &Name::from(segments.remove(0)); // second part is the name of it
        let route = Route::from(segments.join("/"));       // the rest is a sub-route

        debug!("Looking for connection {:?} '{}' called '{}' with route '{}'", direction, object_type, object_name, route);

        match (&direction, object_type) {
            (&Direction::TO, "output") => self.outputs.find_by_name(object_name, &None), // an output from this flow
            (&Direction::FROM, "input") => self.inputs.find_by_name(object_name, &None), // an input to this flow
            (_, "process") => self.get_io_subprocess(object_name, direction, &route, initial_value), // input or output of a sub-process
            _ => bail!("Invalid combination of direction '{:?}' and type '{}' used in connection '{}'",
                             direction, object_type, conn_descriptor)
        }
    }

    /*
        Change the names of connections to be routes to the alias used in this flow,
        in the process ensuring they exist, that direction is correct and types match

        Connection to/from Formats:
            "value/message"
            "input/input_name"
            "output/output_name"

            "process/flow_name/io_name"
            "process/function_name/io_name"

        Propogate any initializers on a flow input into the input (subflow or funcion) it is connected to
    */
    pub fn build_connections(&mut self) -> Result<()> {
        if self.connections.is_none() { return Ok(()); }

        debug!("Building connections for flow '{}'", self.name);

        let mut error_count = 0;

        // get connections out of self - so we can use immutable references to self inside loop
        let connections = replace(&mut self.connections, None);
        let mut connections = connections.unwrap();

        for connection in connections.iter_mut() {
            match self.get_route_and_type(FROM, &connection.from, &None) {
                Ok(from_io) => {
                    debug!("Found connection source:\n{:#?}", from_io);
                    match self.get_route_and_type(TO, &connection.to, from_io.get_initializer()) {
                        Ok(to_io) => {
                            debug!("Found connection destination:\n{:#?}", to_io);
                            if Connection::compatible_types(&from_io, &to_io) {
                                debug!("Connection built from '{}' to '{}'", from_io.route(), to_io.route());
                                connection.from_io = from_io;
                                connection.to_io = to_io;
                            } else {
                                error!("Type mismatch in flow '{}' connection:\n\nfrom\n\n{:#?}\n\nto\n\n{:#?}",
                                       self.source_url, from_io, to_io);
                                error_count += 1;
                            }
                        }
                        Err(error) => {
                            error!("Did not find connection destination: '{}' specified in flow '{}'\n\t\t{}",
                                   connection.to, self.source_url, error);
                            error_count += 1;
                        }
                    }
                }
                Err(error) => {
                    error!("Did not find connection source: '{}' specified in flow '{}'\n\t\t{}",
                           connection.from, self.source_url, error);
                    error_count += 1;
                }
            }
        }

        // put connections back into self
        replace(&mut self.connections, Some(connections));

        if error_count == 0 {
            debug!("All connections inside flow '{}' successfully built", self.source_url);
            Ok(())
        } else {
            bail!("{} connections errors found in flow '{}'", error_count, self.source_url)
        }
    }
}
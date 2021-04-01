use std::collections::{HashMap, HashSet};
use std::fmt;
use std::mem::replace;

use error_chain::bail;
use log::{debug, error};
use serde_derive::{Deserialize, Serialize};
use url::Url;

use flowrstructs::input::InputInitializer;

use crate::compiler::loader::Validate;
use crate::errors::Error;
use crate::errors::*;
use crate::model::connection::Connection;
use crate::model::connection::Direction;
use crate::model::connection::Direction::FROM;
use crate::model::connection::Direction::TO;
use crate::model::io::Find;
use crate::model::io::IOSet;
use crate::model::io::{IOType, IO};
use crate::model::name::HasName;
use crate::model::name::Name;
use crate::model::process::Process;
use crate::model::process::Process::FlowProcess;
use crate::model::process::Process::FunctionProcess;
use crate::model::process_reference::ProcessReference;
use crate::model::route::HasRoute;
use crate::model::route::SetIORoutes;
use crate::model::route::SetRoute;
use crate::model::route::{Route, RouteType};

#[derive(Serialize, Deserialize, Debug, Clone)]
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
    #[serde(rename = "connection")]
    pub connections: Option<Vec<Connection>>,

    #[serde(default = "Flow::default_description")]
    pub description: String,
    #[serde(default = "Flow::default_version")]
    pub version: String,
    #[serde(default = "Flow::default_authors")]
    pub authors: Vec<String>,

    #[serde(skip)]
    pub id: usize,
    #[serde(skip)]
    pub alias: Name,
    #[serde(skip, default = "Flow::default_url")]
    pub source_url: Url,
    #[serde(skip)]
    pub route: Route,
    #[serde(skip)]
    pub subprocesses: HashMap<Name, Process>,
    #[serde(skip)]
    pub lib_references: HashSet<Url>,
}

impl Validate for Flow {
    // check the correctness of all the fields in this flow, prior to loading sub-elements
    fn validate(&self) -> Result<()> {
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
        writeln!(f, "\tname: \t\t\t{}\n\tid: \t\t\t{}\n\talias: \t\t\t{}\n\tsource_url: \t{}\n\troute: \t\t\t{}",
                 self.name, self.id, self.alias, self.source_url, self.route).unwrap();

        writeln!(f, "\tinputs:").unwrap();
        if let Some(ref inputs) = self.inputs {
            for input in inputs {
                writeln!(f, "\t\t\t\t\t{:#?}", input).unwrap();
            }
        }

        writeln!(f, "\toutputs:").unwrap();
        if let Some(ref outputs) = self.outputs {
            for output in outputs {
                writeln!(f, "\t\t\t\t\t{:#?}", output).unwrap();
            }
        }

        writeln!(f, "\tprocesses:").unwrap();
        if let Some(ref process_refs) = self.process_refs {
            for flow_ref in process_refs {
                writeln!(f, "\t{}", flow_ref).unwrap();
            }
        }

        writeln!(f, "\tconnections:").unwrap();
        if let Some(ref connections) = self.connections {
            for connection in connections {
                writeln!(f, "\t\t\t\t\t{}", connection).unwrap();
            }
        }

        Ok(())
    }
}

impl Default for Flow {
    fn default() -> Flow {
        Flow {
            name: Name::default(),
            id: 0,
            alias: Name::default(),
            source_url: Flow::default_url(),
            route: Route::default(),
            process_refs: None,
            inputs: None,
            outputs: None,
            connections: None,
            subprocesses: HashMap::new(),
            lib_references: HashSet::new(),
            description: Flow::default_description(),
            version: Flow::default_version(),
            authors: Flow::default_authors(),
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
    fn route_mut(&mut self) -> &mut Route {
        &mut self.route
    }
}

impl SetRoute for Flow {
    fn set_routes_from_parent(&mut self, parent_route: &Route) {
        if parent_route.is_empty() {
            self.route = Route::from(format!("/{}", self.alias));
        } else {
            self.route = Route::from(format!("{}/{}", parent_route, self.alias));
        }
        self.inputs
            .set_io_routes_from_parent(&self.route, IOType::FlowInput);
        self.outputs
            .set_io_routes_from_parent(&self.route, IOType::FlowOutput);
    }
}

impl Flow {
    fn default_url() -> Url {
        #[allow(clippy::unwrap_used)]
        Url::parse("file://").unwrap()
    }

    pub fn default_description() -> String {
        "".into()
    }

    pub fn default_version() -> String {
        "0.0.0".to_string()
    }

    pub fn default_authors() -> Vec<String> {
        vec!["unknown".to_string()]
    }

    pub fn default_email() -> String {
        "unknown@unknown.com".to_string()
    }

    pub fn set_alias(&mut self, alias: &Name) {
        if alias.is_empty() {
            self.alias = self.name.clone();
        } else {
            self.alias = alias.clone();
        }
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

    fn get_io_subprocess(
        &mut self,
        subprocess_alias: &Name,
        direction: Direction,
        sub_route: &Route,
        initial_value: &Option<InputInitializer>,
    ) -> Result<IO> {
        debug!(
            "\tLooking for subprocess with alias = '{}'",
            subprocess_alias
        );
        let process = self.subprocesses.get_mut(subprocess_alias).ok_or_else(|| {
            Error::from(format!(
                "Could not find sub-process named '{}'",
                subprocess_alias
            ))
        })?;

        // TODO create a trait HasInputs and HasOutputs and implement it for function and flow
        // and process so this below can avoid the match
        match process {
            FlowProcess(ref mut sub_flow) => {
                debug!(
                    "\tFlow sub-process with matching name found, name = '{}'",
                    subprocess_alias
                );
                let io_name = Name::from(sub_route);
                match direction {
                    Direction::TO => sub_flow.inputs.find_by_name(&io_name, initial_value),
                    Direction::FROM => sub_flow.outputs.find_by_name(&io_name, &None),
                }
            }
            FunctionProcess(ref mut function) => {
                debug!(
                    "\tFunction sub-process with matching name found, name = '{}'",
                    subprocess_alias
                );
                match direction {
                    Direction::TO => function.inputs.find_by_route(sub_route, initial_value),
                    Direction::FROM => function.get_outputs().find_by_route(sub_route, &None),
                }
            }
        }
    }

    // TODO consider finding the object first using it's type and name (flow, subflow, value, function)
    // Then from the object find the IO (by name or route, probably route) in common code, maybe using IOSet directly?
    pub fn get_route_and_type(
        &mut self,
        direction: Direction,
        route: &Route,
        initial_value: &Option<InputInitializer>,
    ) -> Result<IO> {
        debug!("Looking for connection {:?} '{}'", direction, route);
        match (&direction, route.route_type()) {
            (&Direction::FROM, RouteType::Input(input_name, sub_route)) => {
                // make sure the sub-route of the input is added to the source of the connection
                let mut from = self.inputs.find_by_name(&input_name, &None)?;
                // accumulate any subroute within the input
                from.route_mut().extend(&sub_route);
                Ok(from)
            }
            (&Direction::TO, RouteType::Output(output_name)) => {
                self.outputs.find_by_name(&output_name, initial_value)
            }
            (_, RouteType::Internal(process_name, sub_route)) => {
                self.get_io_subprocess(&process_name, direction, &sub_route, initial_value)
            }
            (&Direction::FROM, RouteType::Output(output_name)) => {
                bail!("Invalid connection FROM an output named: '{}'", output_name)
            }
            (&Direction::TO, RouteType::Input(input_name, sub_route)) => {
                bail!(
                    "Invalid connection TO an input named: '{}' with sub_route: '{}'",
                    input_name,
                    sub_route
                )
            }
            (_, RouteType::Invalid(error)) => bail!(error),
        }
    }

    /*
        Change the names of connections to be routes to the alias used in this flow,
        in the process ensuring they exist, that direction is correct and types match

        Connection to/from Formats:
            "input/input_name"
            "output/output_name"

            "flow_name/io_name"
            "function_name/io_name"

        Propagate any initializers on a flow input into the input (subflow or function) it is connected to
    */
    pub fn build_connections(&mut self) -> Result<()> {
        if self.connections.is_none() {
            return Ok(());
        }

        debug!("Building connections for flow '{}'", self.name);

        let mut error_count = 0;

        // get connections out of self - so we can use immutable references to self inside loop
        let connections = replace(&mut self.connections, None);

        if let Some(mut conns) = connections {
            for connection in conns.iter_mut() {
                match self.get_route_and_type(FROM, &connection.from, &None) {
                    Ok(from_io) => {
                        debug!("Found connection source:\n{:#?}", from_io);
                        match self.get_route_and_type(TO, &connection.to, from_io.get_initializer())
                        {
                            Ok(to_io) => {
                                debug!("Found connection destination:\n{:#?}", to_io);
                                // TODO here we are only checking compatible data types from the overall FROM IO
                                // not from sub-types in it selected via a sub-route e.g. Array/String --> String
                                // We'd need to make compatible_types more complex and take the from sub-Route
                                if Connection::compatible_types(
                                    &from_io.datatype(),
                                    &to_io.datatype(),
                                ) {
                                    debug!("Connection built from '{}' to '{}' with runtime conversion ''", from_io.route(), to_io.route());
                                    connection.from_io = from_io;
                                    connection.to_io = to_io;
                                } else {
                                    error!("In flow '{}' cannot connect types:\nfrom\n{:#?}\nto\n{:#?}",
                                           self.source_url, from_io, to_io);
                                    error_count += 1;
                                }
                            }
                            Err(error) => {
                                error!("Did not find connection destination: '{}' in flow '{}'\n\t\t{}",
                                       connection.to, self.source_url, error);
                                error_count += 1;
                            }
                        }
                    }
                    Err(error) => {
                        error!(
                            "Did not find connection source: '{}' specified in flow '{}'\n\t\t{}",
                            connection.from, self.source_url, error
                        );
                        error_count += 1;
                    }
                }
            }

            // put connections back into self
            let _ = replace(&mut self.connections, Some(conns));
        }

        if error_count == 0 {
            debug!(
                "All connections inside flow '{}' successfully built",
                self.source_url
            );
            Ok(())
        } else {
            bail!(
                "{} connections errors found in flow '{}'",
                error_count,
                self.source_url
            )
        }
    }
}

#[cfg(test)]
mod test {
    use crate::model::name::{HasName, Name};

    #[test]
    fn test_default_description() {
        assert_eq!("".to_string(), super::Flow::default_description());
    }

    #[test]
    fn test_default_version() {
        assert_eq!("0.0.0".to_string(), super::Flow::default_version());
    }

    #[test]
    fn test_default_authors() {
        assert_eq!(vec!("unknown".to_string()), super::Flow::default_authors());
    }

    #[test]
    fn test_default_email() {
        assert_eq!(
            "unknown@unknown.com".to_string(),
            super::Flow::default_email()
        );
    }

    #[test]
    fn test_display() {
        println!("{}", super::Flow::default());
    }

    #[test]
    fn test_name() {
        let flow = super::Flow::default();
        assert_eq!(flow.name(), &Name::default());
    }

    #[test]
    fn test_alias() {
        let flow = super::Flow::default();
        assert_eq!(flow.alias(), &Name::default());
    }
}

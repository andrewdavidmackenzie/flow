use std::collections::{HashMap, HashSet};
use std::fmt;
use std::mem::{replace, take};

use error_chain::bail;
use log::{debug, error};
use serde_derive::{Deserialize, Serialize};
use url::Url;

use flowcore::deserializers::deserializer::get_deserializer;
use flowcore::flow_manifest::MetaData;
use flowcore::input::InputInitializer;
use flowcore::lib_provider::LibProvider;

use crate::compiler::loader::Validate;
use crate::errors::Error;
use crate::errors::*;
use crate::model::connection::Connection;
use crate::model::connection::Direction;
use crate::model::connection::Direction::FROM;
use crate::model::connection::Direction::TO;
use crate::model::function::Function;
use crate::model::io::Find;
use crate::model::io::IOSet;
use crate::model::io::{IOType, IO};
use crate::model::name::HasName;
use crate::model::name::Name;
use crate::model::process::Process;
use crate::model::process::Process::{FlowProcess, FunctionProcess};
use crate::model::process_reference::ProcessReference;
use crate::model::route::HasRoute;
use crate::model::route::SetIORoutes;
use crate::model::route::SetRoute;
use crate::model::route::{Route, RouteType};

/// `Flow` defines a parent or child flow in the nested flow hierarchy
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct Flow {
    /// `name` given to this flow
    #[serde(rename = "flow")]
    pub name: Name,
    /// `inputs` that this flow defines
    #[serde(default, rename = "input")]
    pub inputs: IOSet,
    /// `outputs` that this flow defines
    #[serde(default, rename = "output")]
    pub outputs: IOSet,
    /// Set of sub-processes referenced (used) in this flow
    #[serde(default, rename = "process")]
    pub process_refs: Vec<ProcessReference>,
    /// `connections` within this flow, from flow input or to flow outputs
    #[serde(default, rename = "connection")]
    pub connections: Vec<Connection>,
    /// `metadata` about flow author, versions etc
    #[serde(default)]
    pub metadata: MetaData,

    /// When the same process is used multiple times within a single flow, to disambiguate
    /// between them each one must be given an alias that is used to refer to it
    #[serde(skip)]
    pub alias: Name,
    /// flows are assigned a numeric `id` in the hierarchy
    #[serde(skip)]
    pub id: usize,
    /// `source_url` is the url of the file/resource where this flow definition was read from
    #[serde(skip, default = "Flow::default_url")]
    pub source_url: Url,
    /// `route` defines the location in the hierarchy of flows where this ones resides
    #[serde(skip)]
    pub route: Route,
    /// `subprocesses` are the loaded definition of the processes reference (used) within this flow
    #[serde(skip)]
    pub subprocesses: HashMap<Name, Process>,
    /// Runtime functions that have already been parsed and loaded - as they were referenced
    #[serde(skip)]
    pub runtime_functions: HashMap<Route, Function>,
    /// `lib_references` is the set of library references used in this flow
    #[serde(skip)]
    pub lib_references: HashSet<Url>,
}

impl Validate for Flow {
    // check the correctness of all the fields in this flow, prior to loading sub-elements
    fn validate(&self) -> Result<()> {
        for input in &self.inputs {
            input.validate()?;
        }

        for output in &self.outputs {
            output.validate()?;
        }

        for connection in &self.connections {
            connection.validate()?;
        }

        Ok(())
    }
}

impl fmt::Display for Flow {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "\tname: \t\t\t{}\n\tid: \t\t\t{}\n\talias: \t\t\t{}\n\tsource_url: \t{}\n\troute: \t\t\t{}",
                 self.name, self.id, self.alias, self.source_url, self.route)?;

        writeln!(f, "\tinputs:")?;
        for input in &self.inputs {
            writeln!(f, "\t\t\t\t\t{:#?}", input)?;
        }

        writeln!(f, "\toutputs:")?;
        for output in &self.outputs {
            writeln!(f, "\t\t\t\t\t{:#?}", output)?;
        }

        writeln!(f, "\tprocesses:")?;
        for flow_ref in &self.process_refs {
            writeln!(f, "\t{}", flow_ref)?;
        }

        writeln!(f, "\tconnections:")?;
        for connection in &self.connections {
            writeln!(f, "\t\t\t\t\t{}", connection)?;
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
            process_refs: vec![],
            inputs: vec![],
            outputs: vec![],
            connections: vec![],
            metadata: MetaData::default(),
            subprocesses: HashMap::new(),
            runtime_functions: HashMap::new(),
            lib_references: HashSet::new(),
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

    /// Set the alias of this flow to the supplied Name
    pub fn set_alias(&mut self, alias: &Name) {
        if alias.is_empty() {
            self.alias = self.name.clone();
        } else {
            self.alias = alias.clone();
        }
    }

    /// Get a reference to the set of inputs this flow defines
    pub fn inputs(&self) -> &IOSet {
        &self.inputs
    }

    /// Get a mutable reference to the set of inputs this flow defines
    pub fn inputs_mut(&mut self) -> &mut IOSet {
        &mut self.inputs
    }

    /// Get a reference to the set of outputs this flow defines
    pub fn outputs(&self) -> &IOSet {
        &self.outputs
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

    fn load_runtime_function(&mut self, url: &Url, provider: &dyn LibProvider) -> Result<Function> {
        let (resolved_url, _) = provider
            .resolve_url(url, "function", &["toml"])
            .chain_err(|| format!("Could not resolve the url: '{}'", url))?;
        debug!("Source URL '{}' resolved to: '{}'", url, resolved_url);

        let contents = provider
            .get_contents(&resolved_url)
            .chain_err(|| format!("Could not get contents of resolved url: '{}'", resolved_url))?;

        let content = String::from_utf8(contents).chain_err(|| "Could not read UTF8 contents")?;
        let deserializer = get_deserializer::<Function>(&resolved_url)?;
        debug!(
            "Loading process from url = '{}' with deserializer: '{}'",
            resolved_url,
            deserializer.name()
        );

        deserializer
            .deserialize(&content, Some(&resolved_url))
            .chain_err(|| {
                format!(
                    "Could not deserialize Function from content in '{}'",
                    resolved_url
                )
            })
    }

    fn insert_runtime_function(
        &mut self,
        route: &Route,
        provider: &dyn LibProvider,
    ) -> Result<Option<&mut Function>> {
        let url = Url::parse(&format!("lib://{}", route))?;

        let function = self.load_runtime_function(&url, provider)?;
        self.runtime_functions.insert(route.clone(), function);

        Ok(self.runtime_functions.get_mut(route))
    }

    fn get_io_runtime_function(
        &mut self,
        direction: Direction,
        route: &Route,
        initial_value: &Option<InputInitializer>,
        provider: &dyn LibProvider,
    ) -> Result<IO> {
        let mut loaded_function = self.runtime_functions.get_mut(route);
        if loaded_function.is_none() {
            loaded_function = self.insert_runtime_function(route, provider)?;
        }

        let function = loaded_function.ok_or("Could not load function")?;

        match direction {
            Direction::TO => function
                .inputs
                .find_by_route_and_set_initializer(route, initial_value),
            Direction::FROM => function
                .get_outputs()
                .find_by_route_and_set_initializer(route, &None)
                .or_else(|_| {
                    function
                        .inputs
                        .find_by_route_and_set_initializer(route, &None)
                }),
        }
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
                    Direction::TO => sub_flow
                        .inputs
                        .find_by_name_and_set_initializer(&io_name, initial_value),
                    Direction::FROM => sub_flow
                        .outputs
                        .find_by_name_and_set_initializer(&io_name, &None),
                }
            }
            FunctionProcess(ref mut function) => {
                debug!(
                    "\tFunction sub-process with matching name found, name = '{}'",
                    subprocess_alias
                );
                match direction {
                    Direction::TO => function
                        .inputs
                        .find_by_route_and_set_initializer(sub_route, initial_value),
                    Direction::FROM => function
                        .get_outputs()
                        .find_by_route_and_set_initializer(sub_route, &None)
                        .or_else(|_| {
                            function
                                .inputs
                                .find_by_route_and_set_initializer(sub_route, &None)
                        }),
                }
            }
        }
    }

    // TODO consider finding the object first using it's type and name (flow, subflow, value, function)
    // Then from the object find the IO (by name or route, probably route) in common code, maybe using IOSet directly?
    /// Find the IO of a function using the route and the Direction of the connection TO/FROM it
    pub fn get_route_and_type(
        &mut self,
        direction: Direction,
        route: &Route,
        initial_value: &Option<InputInitializer>,
        provider: &dyn LibProvider,
    ) -> Result<IO> {
        debug!("Looking for connection {:?} '{}'", direction, route);
        match (&direction, route.route_type()?) {
            (&Direction::FROM, RouteType::Input(input_name, sub_route)) => {
                // make sure the sub-route of the input is added to the source of the connection
                let mut from = self
                    .inputs
                    .find_by_name_and_set_initializer(&input_name, &None)?;
                // accumulate any subroute within the input
                from.route_mut().extend(&sub_route);
                Ok(from)
            }
            (&Direction::TO, RouteType::Output(output_name)) => self
                .outputs
                .find_by_name_and_set_initializer(&output_name, initial_value),
            (_, RouteType::Internal(process_name, sub_route)) => {
                self.get_io_subprocess(&process_name, direction, &sub_route, initial_value)
            }
            (_, RouteType::FlowRunTime(_)) => {
                self.get_io_runtime_function(direction, route, initial_value, provider)
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
        }
    }

    /// Change the names of connections to be routes to the alias used in this flow,
    /// in the process ensuring they exist, that direction is correct and types match
    ///
    /// Connection to/from Formats:
    /// "input/input_name"
    /// "output/output_name"
    ///
    /// "flow_name/io_name"
    /// "function_name/io_name"
    ///
    /// Propagate any initializers on a flow input into the input (subflow or function) it is connected to
    pub fn build_connections(&mut self, provider: &dyn LibProvider) -> Result<()> {
        if self.connections.is_empty() {
            return Ok(());
        }

        debug!("Building connections for flow '{}'", self.name);

        let mut error_count = 0;

        // get connections out of self - so we can use immutable references to self inside loop
        let mut connections = take(&mut self.connections);

        for connection in connections.iter_mut() {
            match self.get_route_and_type(FROM, &connection.from, &None, provider) {
                Ok(from_io) => {
                    debug!("Found connection source:\n{:#?}", from_io);
                    match self.get_route_and_type(
                        TO,
                        &connection.to,
                        from_io.get_initializer(),
                        provider,
                    ) {
                        Ok(to_io) => {
                            debug!("Found connection destination:\n{:#?}", to_io);
                            // TODO here we are only checking compatible data types from the overall FROM IO
                            // not from sub-types in it selected via a sub-route e.g. Array/String --> String
                            // We'd need to make compatible_types more complex and take the from sub-Route
                            if Connection::compatible_types(from_io.datatype(), to_io.datatype()) {
                                debug!(
                                    "Connection built from '{}' to '{}' with runtime conversion ''",
                                    from_io.route(),
                                    to_io.route()
                                );
                                connection.from_io = from_io;
                                connection.to_io = to_io;
                            } else {
                                error!(
                                    "In flow '{}' cannot connect types:\nfrom\n{:#?}\nto\n{:#?}",
                                    self.source_url, from_io, to_io
                                );
                                error_count += 1;
                            }
                        }
                        Err(error) => {
                            error!(
                                "Did not find connection destination: '{}' in flow '{}'\n\t\t{}",
                                connection.to, self.source_url, error
                            );
                            error_count += 1;
                        }
                    }
                }
                Err(error) => {
                    error!(
                        "Did not find connection source: '{}' specified in flow '{}'\n\t{}",
                        connection.from, self.source_url, error
                    );
                    error_count += 1;
                }
            }
        }

        // put connections back into self
        let _ = replace(&mut self.connections, connections);

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

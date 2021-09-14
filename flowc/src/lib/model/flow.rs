use std::collections::{HashMap, HashSet};
use std::fmt;
use std::mem::{replace, take};

use error_chain::bail;
use log::{debug, error};
use serde_derive::{Deserialize, Serialize};
use url::Url;

use flowcore::flow_manifest::MetaData;
use flowcore::input::InputInitializer;

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
            subprocesses: HashMap::new(),
            lib_references: HashSet::new(),
            metadata: MetaData::default(),
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
    ) -> Result<IO> {
        debug!("Looking for connection {:?} '{}'", direction, route);
        match (&direction, route.route_type()) {
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

    // Connection to/from Formats:
    // "input/input_name"
    // "output/output_name"
    //
    // "flow_name/io_name"
    // "function_name/io_name"
    //
    // Propagate any initializers on a flow output to the input (subflow or function) it is connected to
    fn build_connection(&mut self, connection: &mut Connection) -> Result<()> {
        match self.get_route_and_type(FROM, &connection.from, &None) {
            Ok(from_io) => {
                debug!("Found connection source:\n{:#?}", from_io);
                match self.get_route_and_type(TO, &connection.to, from_io.get_initializer()) {
                    Ok(to_io) => {
                        debug!("Found connection destination:\n{:#?}", to_io);
                        if Connection::compatible_types(
                            from_io.datatype(),
                            to_io.datatype(),
                            &connection.from,
                        ) {
                            debug!(
                                "Connection built from '{}' to '{}' with runtime conversion ''",
                                from_io.route(),
                                to_io.route()
                            );
                            connection.from_io = from_io;
                            connection.to_io = to_io;
                            Ok(())
                        } else {
                            bail!(
                                "In flow '{}' cannot connect types:\nfrom\n{:#?}\nto\n{:#?}",
                                self.source_url,
                                from_io,
                                to_io
                            );
                        }
                    }
                    Err(error) => {
                        bail!(
                            "Did not find connection destination: '{}' in flow '{}'\n\t\t{}",
                            connection.to,
                            self.source_url,
                            error
                        );
                    }
                }
            }
            Err(error) => {
                bail!(
                    "Did not find connection source: '{}' specified in flow '{}'\n\t{}",
                    connection.from,
                    self.source_url,
                    error
                );
            }
        }
    }

    /// Iterate over all the connections defined in the flow, and attempt to connect the source
    /// and destination, checking the types are compatible
    pub fn build_connections(&mut self) -> Result<()> {
        debug!("Building connections for flow '{}'", self.name);

        let mut error_count = 0;

        // get connections out of self - so we can use immutable references to self inside loop
        let mut connections = take(&mut self.connections);

        for connection in connections.iter_mut() {
            if let Err(e) = self.build_connection(connection) {
                error_count += 1;
                error!("{}", e);
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
    use std::collections::HashMap;

    use serde_json::json;

    use flowcore::input::InputInitializer::Always;
    use flowcore::input::InputInitializer::Once;

    use crate::compiler::loader::Validate;
    use crate::model::connection::Connection;
    use crate::model::flow::Flow;
    use crate::model::function::Function;
    use crate::model::io::IO;
    use crate::model::name::{HasName, Name};
    use crate::model::process::Process;
    use crate::model::route::{HasRoute, Route, SetRoute};

    // Create a test flow we can use in connection building testing
    fn test_flow() -> Flow {
        let mut flow = Flow {
            name: "test_flow".into(),
            alias: "test_flow".into(),
            inputs: vec![
                IO {
                    datatype: "String".into(),
                    route: "string".into(),
                    name: "string".into(),
                    ..Default::default()
                },
                IO {
                    datatype: "Number".into(),
                    route: "number".into(),
                    name: "number".into(),
                    ..Default::default()
                },
            ],
            outputs: vec![
                IO {
                    datatype: "String".into(),
                    route: "string".into(),
                    name: "string".into(),
                    ..Default::default()
                },
                IO {
                    datatype: "Number".into(),
                    route: "number".into(),
                    name: "number".into(),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        let process_1 = Process::FunctionProcess(Function {
            name: "process_1".into(),
            id: 0,
            inputs: vec![IO::new("String", "")],
            outputs: vec![IO::new("String", "")],
            ..Default::default()
        });

        let process_2 = Process::FunctionProcess(Function {
            name: "process_2".into(),
            id: 1,
            inputs: vec![IO::new("String", "")],
            outputs: vec![IO::new("Number", "")],
            ..Default::default()
        });

        let _ = flow.subprocesses.insert("process_1".into(), process_1);
        let _ = flow.subprocesses.insert("process_2".into(), process_2);

        flow
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

    #[test]
    fn test_set_alias() {
        let mut flow = super::Flow::default();
        flow.set_alias(&Name::from("test flow"));
        assert_eq!(flow.alias(), &Name::from("test flow"));
    }

    #[test]
    fn test_set_empty_alias() {
        let mut flow = super::Flow::default();
        flow.set_alias(&Name::from(""));
        assert_eq!(flow.alias(), &Name::from(""));
    }

    #[test]
    fn test_route() {
        let flow = super::Flow::default();
        assert_eq!(flow.route(), &Route::default());
    }

    #[test]
    fn test_route_mut() {
        let mut flow = super::Flow::default();
        let route = flow.route_mut();
        assert_eq!(route, &Route::default());
        *route = Route::from("/context");
        assert_eq!(route, &Route::from("/context"));
    }

    #[test]
    fn test_set_empty_parent_route() {
        let mut flow = test_flow();
        flow.set_routes_from_parent(&Route::from(""));
        assert_eq!(flow.route(), &Route::from("/test_flow"));
    }

    #[test]
    fn test_set_parent_route() {
        let mut flow = test_flow();
        flow.set_routes_from_parent(&Route::from("/context"));
        assert_eq!(flow.route(), &Route::from("/context/test_flow"));
    }

    #[test]
    fn validate_flow() {
        let mut flow = test_flow();
        let connection = Connection {
            from: "process_1".into(), // String
            to: "process_2".into(),   // String
            ..Default::default()
        };
        flow.connections = vec![connection];
        assert!(flow.validate().is_ok());
    }

    #[test]
    fn check_outputs() {
        let flow = test_flow();
        assert_eq!(flow.outputs().len(), 2);
    }

    #[test]
    fn check_inputs() {
        let flow = test_flow();
        assert_eq!(flow.inputs().len(), 2);
    }

    #[test]
    fn check_inputs_mut() {
        let mut flow = test_flow();
        let inputs = flow.inputs_mut();
        assert_eq!(inputs.len(), 2);
        *inputs = vec![];
        assert_eq!(inputs.len(), 0);
    }

    #[test]
    fn test_inputs_initializers() {
        let mut flow = test_flow();
        let mut initializers = HashMap::new();
        initializers.insert("string".into(), Always(json!("Hello")));
        initializers.insert("number".into(), Once(json!(42)));
        flow.set_initial_values(&initializers);

        assert_eq!(
            flow.inputs()
                .get(0)
                .expect("Could not get input")
                .get_initializer()
                .as_ref()
                .expect("Could not get initializer"),
            &Always(json!("Hello"))
        );

        assert_eq!(
            flow.inputs()
                .get(1)
                .expect("Could not get input")
                .get_initializer()
                .as_ref()
                .expect("Could not get initializer"),
            &Once(json!(42))
        );
    }

    #[test]
    fn display_flow() {
        let mut flow = test_flow();
        let connection = Connection {
            from: "process_1".into(), // String
            to: "process_2".into(),   // String
            ..Default::default()
        };
        flow.connections = vec![connection];
        println!("flow: {}", flow);
    }

    mod build_connection_tests {
        use crate::model::connection::Connection;
        use crate::model::flow::test::test_flow;

        #[test]
        fn build_compatible_internal_connection() {
            let mut flow = test_flow();

            let mut connection = Connection {
                from: "process_1".into(), // String
                to: "process_2".into(),   // String
                ..Default::default()
            };

            assert!(flow.build_connection(&mut connection).is_ok());
        }

        #[test]
        fn build_incompatible_internal_connection() {
            let mut flow = test_flow();

            let mut connection = Connection {
                from: "process_2".into(), // Number
                to: "process_1".into(),   // String
                ..Default::default()
            };

            assert!(flow.build_connection(&mut connection).is_err());
        }

        #[test]
        fn build_from_flow_input_to_sub_process() {
            let mut flow = test_flow();

            let mut connection = Connection {
                from: "input/string".into(), // String
                to: "process_1".into(),      // String
                ..Default::default()
            };

            assert!(flow.build_connection(&mut connection).is_ok());
        }

        #[test]
        fn build_from_sub_process_flow_output() {
            let mut flow = test_flow();

            let mut connection = Connection {
                from: "process_1".into(),   // String
                to: "output/string".into(), // String
                ..Default::default()
            };

            assert!(flow.build_connection(&mut connection).is_ok());
        }

        #[test]
        fn build_from_flow_input_to_flow_output() {
            let mut flow = test_flow();

            let mut connection = Connection {
                from: "input/string".into(), // String
                to: "output/string".into(),  // String
                ..Default::default()
            };

            assert!(flow.build_connection(&mut connection).is_ok());
        }

        #[test]
        fn build_incompatible_from_flow_input_to_sub_process() {
            let mut flow = test_flow();

            let mut connection = Connection {
                from: "input/number".into(), // Number
                to: "process_1".into(),      // String
                ..Default::default()
            };

            assert!(flow.build_connection(&mut connection).is_err());
        }

        #[test]
        fn build_incompatible_from_sub_process_flow_output() {
            let mut flow = test_flow();

            let mut connection = Connection {
                from: "process_1".into(),   // String
                to: "output/number".into(), // Number
                ..Default::default()
            };

            assert!(flow.build_connection(&mut connection).is_err());
        }

        #[test]
        fn build_incompatible_from_flow_input_to_flow_output() {
            let mut flow = test_flow();

            let mut connection = Connection {
                from: "input/string".into(), // String
                to: "output/number".into(),  // Number
                ..Default::default()
            };

            assert!(flow.build_connection(&mut connection).is_err());
        }

        #[test]
        fn fail_build_from_flow_input_to_flow_input() {
            let mut flow = test_flow();

            let mut connection = Connection {
                from: "input/string".into(), // String
                to: "input/number".into(),   // Number
                ..Default::default()
            };

            assert!(flow.build_connection(&mut connection).is_err());
        }

        #[test]
        fn fail_build_from_flow_output_to_flow_output() {
            let mut flow = test_flow();

            let mut connection = Connection {
                from: "output/string".into(), // String
                to: "output/number".into(),   // Number
                ..Default::default()
            };

            assert!(flow.build_connection(&mut connection).is_err());
        }

        #[test]
        fn build_all_flow_connections() {
            let mut flow = test_flow();

            let connection1 = Connection {
                from: "input/string".into(), // String
                to: "output/string".into(),  // String
                ..Default::default()
            };

            let connection2 = Connection {
                from: "input/string".into(), // String
                to: "process_1".into(),      // String
                ..Default::default()
            };

            let connection3 = Connection {
                from: "process_1".into(),   // String
                to: "output/string".into(), // String
                ..Default::default()
            };

            flow.connections = vec![connection1, connection2, connection3];
            assert!(flow.build_connections().is_ok());
        }

        #[test]
        fn fail_build_flow_connections() {
            let mut flow = test_flow();

            let connection1 = Connection {
                from: "input/number".into(), // Number
                to: "process_1".into(),      // String
                ..Default::default()
            };

            flow.connections = vec![connection1];
            assert!(flow.build_connections().is_err());
        }
    }
}

use model::name::Name;
use model::name::HasName;
use model::datatype::DataType;
use model::connection::HasRoute;
use model::connection::Connection;
use model::datatype::HasDataType;
use model::input::Input;
use model::output::Output;
use model::value::Value;
use model::flow_reference::FlowReference;
use model::connection::Route;
use loader::loader::Validate;
use model::function_reference::FunctionReference;
use model::connection::Direction;
use std::fmt;
use url::Url;

#[derive(Deserialize)]
pub struct Flow {
    pub name: Name,
    #[serde(rename = "flow")]
    pub flow_refs: Option<Vec<FlowReference>>,
    #[serde(rename = "function")]
    pub function_refs: Option<Vec<FunctionReference>>,
    #[serde(rename = "value")]
    pub values: Option<Vec<Value>>,
    #[serde(rename = "input")]
    pub inputs: Option<Vec<Input>>,
    #[serde(rename = "output")]
    pub outputs: Option<Vec<Output>>,
    #[serde(rename = "connection")]
    pub connections: Option<Vec<Connection>>,

    #[serde(skip_deserializing, default = "Flow::default_url")]
    pub source_url: Url,
    #[serde(skip_deserializing)]
    pub route: Route,
    #[serde(skip_deserializing)]
    pub lib_references: Vec<String>,
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

        if let Some(ref flows_refs) = self.flow_refs {
            for flow_ref in flows_refs {
                flow_ref.validate()?;
            }
        }

        if let Some(ref function_refs) = self.function_refs {
            for function_ref in function_refs {
                function_ref.validate()?;
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
        write!(f, "\tname: \t\t\t{}\n\tsource_url: \t\t{}\n\troute: \t\t\t{}\n",
               self.name, self.source_url, self.route).unwrap();

        // TODO dry this all up now it works.

        write!(f, "\tvalues:\n").unwrap();
        if let Some(ref values) = self.values {
            for value in values {
                write!(f, "\t\t\t\t{}\n", value).unwrap();
            }
        }

        write!(f, "\tinputs:\n").unwrap();
        if let Some(ref inputs) = self.inputs {
            for input in inputs {
                write!(f, "\t\t\t\t\t{}\n", input).unwrap();
            }
        }

        write!(f, "\touputs:\n").unwrap();
        if let Some(ref outputs) = self.outputs {
            for output in outputs {
                write!(f, "\t\t\t\t\t{}\n", output).unwrap();
            }
        }

        write!(f, "\tsubflows:\n").unwrap();
        if let Some(ref flow_refs) = self.flow_refs {
            for flow_ref in flow_refs {
                write!(f, "\t{}\n", flow_ref).unwrap();
            }
        }

        write!(f, "\tfunctions: \t\n").unwrap();
        if let Some(ref function_refs) = self.function_refs {
            for function_ref in function_refs {
                write!(f, "\t{}", function_ref).unwrap();
                write!(f, "\t{}", function_ref.function).unwrap();
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
            source_url: Flow::default_url(),
            name: "".to_string(),
            route: "".to_string(),
            flow_refs: None,
            function_refs: None,
            values: None,
            inputs: None,
            outputs: None,
            connections: None,
            lib_references: vec!(),
        }
    }
}

impl Flow {
    fn default_url() -> Url {
        Url::parse("file:///").unwrap()
    }

    /*
        Set the routes of inputs and outputs in a flow to the hierarchical format
    */
    pub fn set_io_routes(&mut self) {
        if let &mut Some(ref mut ios) = &mut self.inputs {
            debug!("Setting Input routes for flow '{}'", self.source_url);
            for ref mut input in ios {
                input.route = format!("{}/{}", self.route, input.name);
                debug!("Input route: '{}'", input.route);
            }
        }

        if let &mut Some(ref mut ios) = &mut self.outputs {
            debug!("Setting Output routes for flow '{}'", self.source_url);
            for ref mut output in ios {
                output.route = format!("{}/{}", self.route, output.name);
                debug!("Output route: '{}'", output.route);
            }
        }
    }

    // Look through a collection of inputs, or outputs, to find one by name and return it's
    // route and datatype and if starts/ends at a flow boundary
    fn get<E: HasName + HasRoute + HasDataType>(&self,
                                                collection: &Option<Vec<E>>,
                                                element_name: &str)
                                                -> Result<(Route, DataType, bool), String> {
        if let &Some(ref elements) = collection {
            for element in elements {
                if element.name() == element_name {
                    return Ok((format!("{}", element.route()),
                               format!("{}", element.datatype()),
                               true));
                }
            }
            return Err(format!("No inout or output with name '{}' was found", element_name));
        }
        Err(format!("No inputs or outputs found when looking for input/output '{}'", element_name))
    }

    fn get_io_subflow(&self, subflow_alias: &str, direction: Direction, io_name: &str)
                      -> Result<(Route, DataType, bool), String> {
        if let Some(ref flow_refs) = self.flow_refs {
            for flow_ref in flow_refs {
                if flow_ref.name() == subflow_alias {
                    return match direction {
                        Direction::TO => flow_ref.flow.get(&flow_ref.flow.inputs, io_name),
                        Direction::FROM => flow_ref.flow.get(&flow_ref.flow.outputs, io_name)
                    };
                }
            }
            return Err(format!("Could not find subflow named '{}'", subflow_alias));
        }

        return Err("No subflows present".to_string());
    }

    fn get_io_function(&self, function_alias: &str, direction: Direction, route: &str) -> Result<(Route, DataType, bool), String> {
        if let Some(ref function_refs) = self.function_refs {
            for function_ref in function_refs {
                if function_ref.name() == function_alias {
                    return match direction {
                        Direction::TO => function_ref.function.get(&function_ref.function.inputs, route),
                        Direction::FROM => function_ref.function.get(&function_ref.function.outputs, route)
                    };
                }
            }
            return Err(format!("Could not find function named '{}' in flow '{}'",
                               function_alias, self.name));
        }

        return Err(format!("No functions present in flow '{}'. Could not find route '{}'",
                           self.name, route));
    }

    fn get_io_value(&self, value_name: &str, direction: Direction, route: &str) -> Result<(Route, DataType, bool), String> {
        if let &Some(ref values) = &self.values {
            for value in values {
                if value.name == value_name {
                    return match direction {
                        Direction::TO => Ok((value.route.clone(), value.datatype.clone(), false)),
                        Direction::FROM => value.get_output(route)
                    };
                }
            }
            return Err(format!("Could not find value named '{}'", value_name));
        }

        return Err("No values present".to_string());
    }

    pub fn get_route_and_type(&mut self, direction: Direction, conn_descriptor: &str) -> Result<(Route, DataType, bool), String> {
        let mut segments: Vec<&str> = conn_descriptor.split('/').collect();
        let object_type = segments.remove(0); // first part is type of object
        let object_name = segments.remove(0); // second part is the name of it
        let route = segments.join("/");       // the rest is a sub-route

        debug!("Looking for connection {:?} {} '{}' with sub-route '{}'", direction, object_type, object_name, route);

        match (&direction, object_type) {
            (&Direction::TO, "output") => self.get(&self.outputs, object_name), // an output from this flow
            (&Direction::FROM, "input") => self.get(&self.inputs, object_name), // an input to this flow
            (_, "flow") => self.get_io_subflow(object_name, direction, &route), // input or output of a subflow
            (_, "value") => self.get_io_value(object_name, direction, &route), // input or output of a contained value
            (_, "function") => self.get_io_function(object_name, direction, &route), // input or output of a referenced function
            _ => Err(format!("Unknown type of object '{}' used in IO descriptor '{}'", object_type, conn_descriptor))
        }
    }
}
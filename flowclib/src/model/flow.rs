use model::name::Name;
use model::name::HasName;
use model::datatype::DataType;
use model::connection::HasRoute;
use model::connection::Connection;
use model::datatype::HasDataType;
use model::io::IO;
use model::value::Value;
use model::flow_reference::FlowReference;
use model::connection::Route;
use loader::loader::Validate;
use model::function_reference::FunctionReference;
use std::fmt;
use url::Url;

#[derive(Deserialize, Debug)]
pub struct Flow {
    #[serde(skip_deserializing, default = "Flow::default_url")]
    pub source_url: Url,
    pub name: Name,
    #[serde(skip_deserializing)]
    pub route: Route,

    #[serde(rename = "flow")]
    pub flow_refs: Option<Vec<FlowReference>>,
    #[serde(rename = "function")]
    pub function_refs: Option<Vec<FunctionReference>>,

    #[serde(rename = "value")]
    pub values: Option<Vec<Value>>,

    #[serde(rename = "input")]
    pub inputs: Option<Vec<IO>>,
    #[serde(rename = "output")]
    pub outputs: Option<Vec<IO>>,

    #[serde(rename = "connection")]
    pub connections: Option<Vec<Connection>>,

    #[serde(skip_deserializing)]
    pub libs: Vec<String>,
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
            libs: vec!(),
            lib_references: vec!()
        }
    }
}

impl Flow {
    fn default_url() -> Url {
        Url::parse("file:///").unwrap()
    }

    /*
        Set the routes of inputs and outputs in a flow to the hierarchical format
        using the internal name of the thing referenced
    */
    pub fn set_io_routes(&mut self) {
        if let &mut Some(ref mut ios) = &mut self.inputs {
            for ref mut io in ios {
                io.route = format!("{}/{}", self.route, io.name);
            }
        }
        if let &mut Some(ref mut ios) = &mut self.outputs {
            for ref mut io in ios {
                io.route = format!("{}/{}", self.route, io.name);
            }
        }
    }

    fn get<E: HasName + HasRoute + HasDataType>(&self,
                                                collection: &Option<Vec<E>>,
                                                element_name: &str, flow: bool)
                                                -> Result<(Route, DataType, bool), String> {
        if let &Some(ref elements) = collection {
            for element in elements {
                if element.name() == element_name {
                    return Ok((format!("{}", element.route()),
                               format!("{}", element.datatype()),
                               flow));
                }
            }
            return Err(format!("No output with name '{}' was found", element_name));
        }
        Err(format!("No outputs found."))
    }

    /*
        Find an io of the specified "direction" ("value", "input" or "output") and "name"
        within the flow.
    */
    fn get_io(&self, direction: &str, name: &str)
              -> Result<(Route, DataType, bool), String> {
        match direction {
            "value" => self.get(&self.values, name, false),
            "input" => self.get(&self.inputs, name, true),
            "output" => self.get(&self.outputs, name, true),
            _ => Err(format!("Could not find name '{}' in '{}'", name, self.name))
        }
    }

    // TODO Combine these two using a reference trait get_reference_io or just "flow" or function" switch
    fn get_io_subflow(&self, subflow_alias: &str, io_name: &str)
                      -> Result<(Route, DataType, bool), String> {
        if let Some(ref flow_refs) = self.flow_refs {
            for flow_ref in flow_refs {
                if flow_ref.name() == subflow_alias {
                    // TODO There's probably a way to do this better using or_else
                    let found = flow_ref.flow.get_io("input", io_name);
                    if found.is_ok() {
                        return found;
                    }
                    return flow_ref.flow.get_io("output", io_name);
                }
            }
            return Err(format!("Could not find subflow named '{}'", subflow_alias));
        }

        return Err("No subflows present".to_string());
    }

    fn get_io_function(&self, function_alias: &str, io_name: &str)
                       -> Result<(Route, DataType, bool), String> {
        if let Some(ref function_refs) = self.function_refs {
            for function_ref in function_refs {
                if function_ref.name() == function_alias {
                    let found = function_ref.get_io("input", io_name);
                    if found.is_ok() {
                        return found;
                    }
                    return function_ref.get_io("output", io_name);
                }
            }
            return Err(format!("Could not find function named '{}'", function_alias));
        }

        return Err("No functions present".to_string());
    }

    pub fn get_route_and_type(&mut self, conn_descriptor: &str) -> Result<(Route, DataType, bool), String> {
        let segments: Vec<&str> = conn_descriptor.split('/').collect();

        match segments.len() {
            2 => self.get_io(segments[0], segments[1]),

            3 => match (segments[0], segments[1], segments[2]) {
                ("flow", flow_alias, io_name) => self.get_io_subflow(flow_alias, io_name),
                ("function", function_alias, io_name) => self.get_io_function(function_alias, io_name),
                (_, _, _) => Err(format!("Invalid name '{}' used in connection", conn_descriptor))
            },

            _ => Err(format!("Invalid name format '{}' used in connection", conn_descriptor))
        }
    }
}
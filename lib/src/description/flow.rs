use loader::loader::Validate;
use description::name::Name;
use description::entity::EntityRef;
use description::connection::Connection;
use std::fmt;

#[derive(Deserialize, Debug)]
pub struct FlowRef {
    pub name: Name,
    pub source: String
}

impl fmt::Display for FlowRef {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Flow:\n\tname: {}\n\tsource: {}", self.name, self.source)
    }
}

#[derive(Deserialize)]
pub struct Flow {
    pub name: Name,
    pub flow: Vec<FlowRef>,
    pub entity: Vec<EntityRef>,
    pub connection: Vec<Connection>

    //    pub _flow: Vec<Box<Flow>>,
    /*
	functions: Vec<Function>,
    values: Vec<Value>,
	ios: IOSet,
*/
}

impl Flow {
    pub fn new(name: Name,
               flow: Vec<FlowRef>,
               entity: Vec<EntityRef>,
               connection: Vec<Connection>) -> Flow {
        Flow {
            name: name,
            flow: flow,
            entity: entity,
            connection: connection
            /*
            entities: entities,
            values: values,
            connection_set: connection_set,
            */
        }
    }
}

/*
Validate the correctness of all the fields in this flow,
but not consistency with contained flows
 */
impl Validate for Flow {
    fn validate(&self) -> Result<(), String> {
        self.name.validate() // TODO early return
        /*
                for entity in &self.entities {
                    entity.validate(); // TODO early return
                }

                for value in &self.values {
                    value.validate(); // TODO early return
                }

                //            context.load_sub_flows(); // TODO early return
                //            context.validate_connections(); // TODO early return
                //            for &(_, _, ref subflow) in context.flows.iter() {
                //                subflow.borrow_mut().subflow(); // TODO early return
                //            }


                if self.flows.len() > 1 as usize {
                    return Err("context: cannot contain more than one sub-flow".to_string());
                }

                for &(ref name, _, _) in self.flows.iter() {
                    name.validate("Flow");
                }

                self.connection_set.validate()*/


        /*
        let mut io_sets: Vec<&IOSet> = vec![];

        for &(_, _, ref flow) in self.flows.iter() {
            // add subflow's ioset to the set to check connections to
            // TODO FIX
            //            io_sets.push(&(flow.borrow_mut().ios));
        }

        for entity in &self.entities {
            io_sets.push(&entity.ios);
        }

        // TODO
        // for each connection
        // connected at both ends to something in this Context
        // 		validateConnection in itself, not to subflow

        // for each check connections with their ioset
        ConnectionSet::check(&self.connection_set, &io_sets, &self.values)
        */
    }
}

impl fmt::Display for Flow {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "name: {}\nflow: {:?}\nentity: {:?}\nconnection: {:?}", self.name, self.flow,
               self.entity, self.connection)
    }
}

//            flow.validate(); // TODO early return
//            flow.load_sub_flows(); // TODO early return
//            flow.validate_connections(); // TODO early return
//            flow.subflow();

/*
impl Flow {
	pub fn new(name: String, path: &str, flows: Vec<(String, String, Box<Flow>)>,
		   connection_set: ConnectionSet, ios: IOSet, values: Vec<Value>, functions: Vec<Function>)
	-> Flow {
		Flow {
			name: name,
			source_path: path.to_string(),
			flows: flows,
			ios: ios,
			values: values,
			functions: functions,
			connection_set: connection_set,
		}
	}

	pub fn validate_fields(&self) -> parser::Result {
		self.name.validate_fields("Flow"); // TODO early return

		// validate flows (name only and valid path)

		// Validate all IOs are valid names and types

		// Validate values

		// validate functions

		// validate connections are all valid

        // TODO
        parser::Result::Valid
	}

    pub fn load_sub_flows(&mut self) -> parser::Result {
        for &(_, ref path, _) in self.flows.iter() {
            // TODO FIX
            let load_result = parser::load(path.as_ref(), false);
//            let load_result = parser::Result::Valid;
            match load_result {
                parser::Result::FlowLoaded(subflow) => {
                    // TODO set reference to child flow
                    return parser::Result::Valid;
                },
                _ => {},
            }
            return load_result;
        }
        parser::Result::Valid
    }

    pub fn validate_connections(&self) -> parser::Result {
        let mut io_sets: Vec<&IOSet> = vec![];

        for &(_, _, ref flow) in self.flows.iter() {
            // add subflow's ioset to the set of IOs to check connections to
            io_sets.push(&(flow.ios));
        }

        // Add the IOSets of all functions
        for function in &self.functions {
            io_sets.push(&function.ios);
        }

        // Add the input/outputs of this flow to parent
        io_sets.push(&self.ios);

        // for each check connections with their ioset
        ConnectionSet::check(&self.connection_set, &io_sets, &self.values)
    }

    pub fn subflow(&mut self) -> parser::Result {
        for &mut(_, _, ref mut subflow) in self.flows.iter_mut() {
            subflow.validate_fields(); // TODO early return
            subflow.load_sub_flows(); // TODO early return
            subflow.validate_connections(); // TODO early return
            subflow.subflow();
        }

        // TODO FIX
        parser::Result::Valid
    }
}
*/
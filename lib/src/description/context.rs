use parser::parser::Validate;

use description::name::Name;
use description::entity::Entity;
use description::connection::ConnectionSet;
use description::flow::Flow;
use description::value::Value;
use description::io::IOSet;

use std::cell::RefCell;

pub struct Context {
	pub name: Name,
    /*
    source_path: String,
    entities: Vec<Entity<'a>>,
    pub flows: Vec<(Name<'a>, String, RefCell<Flow<'a>>)>,
    values: Vec<Value<'a>>,
    connection_set: ConnectionSet<'a>,
*/
}

/*
Validate the correctness of all the fields in this context,
but not consistency with contained flows
 */
impl Validate for Context {
    fn validate(&self) -> Result<(), String> {
        self.name.validate() // TODO early return
/*
        for entity in &self.entities {
            entity.validate(); // TODO early return
        }

        for value in &self.values {
            value.validate(); // TODO early return
        }

        if self.flows.len() > 1 as usize {
            return Err("context: cannot contain more than one sub-flow".to_string());
        }

        for &(ref name, _, _) in self.flows.iter() {
            name.validate("Flow");
        }

        self.connection_set.validate()*/
    }
}

/*
impl Context {
	pub fn new(name: Name, path: &str, entities: Vec<Entity>, values: Vec<Value>,
		   flows: Vec<(Name, String, RefCell<Flow>)>, connection_set: ConnectionSet ) -> Context {
		Context {
			name: name,
			source_path: path.to_string(),
			entities: entities,
			values: values,
			flows: flows,
			connection_set: connection_set,
		}
	}

    pub fn load_sub_flows(&self) -> parser::Result {
        for &(_, ref path, _) in self.flows.iter() {
            // load subflows
            let load_result = parser::load(path.as_ref(), false);
            match load_result {
                parser::Result::FlowLoaded(subflow) => {
                    // TODO set reference to child flow
                },
                _ => return load_result,
            }
        }
        parser::Result::Valid
    }

    pub fn validate_connections(&self) -> parser::Result {
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
    }
}
*/

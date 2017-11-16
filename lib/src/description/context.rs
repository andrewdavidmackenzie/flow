use std::fmt;

use loader::loader::Validate;
use description::name::Name;
use description::flow::Flow;
use std::path::PathBuf;

/*
use description::entity::Entity;
use description::connection::ConnectionSet;
use description::flow::Flow;
use description::value::Value;
use description::io::IOSet;
*/

pub struct Context {
    pub source: String,
    pub name: Name,
    /*
    source_path: String,
    entities: Vec<Entity<'a>>,
    */
    pub flow: Option<Box<Flow>>,
    /*
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
    }
}

impl Context {
    pub fn new(source: PathBuf, name: Name, /* entities: Vec<Entity>, values: Vec<Value>, */
               flow: Option<Box<Flow>> /*, connection_set: ConnectionSet */) -> Context {
        Context {
            source: source.to_str().unwrap().to_string(),
            name: name,
            flow: flow
            /*
            source_path: path.to_string(),
            entities: entities,
            values: values,
            connection_set: connection_set,
            */
        }
    }
}

impl fmt::Display for Context {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Source: {}\nName: {}", self.source, self.name)
    }
}

impl Context {
    fn validate_connections(&self) -> Result<(), String> {
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
        Ok(())
    }
}
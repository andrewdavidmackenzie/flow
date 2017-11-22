use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::io::prelude::*;
use toml;

/* use description::flow::Flow;
use description::entity::Entity;
use description::io::IOSet;
use description::value::Value;
use description::function::Function;
use description::connection::ConnectionSet;*/

use loader::loader::Loader;
use description::flow::Flow;

pub struct FlowTomelLoader{}

/*
 load the toml file
 */
impl Loader for FlowTomelLoader {
    // TODO define our own errors types?
    fn load(file_path: &PathBuf) -> Result<Flow, String> {
        let file = File::open(file_path).unwrap(); // TODO handle error
        let mut buf_reader = BufReader::new(file);
        let mut contents = String::new();
        match buf_reader.read_to_string(&mut contents) {
            Ok(_) => {
                match toml::from_str(&contents) {
                    Ok(flow) => {
                        load_flow_contents(&flow);
                        // TODO figure out how to return Ok or Err with the flow in it from contents
                        Ok(flow)
                    },
                    Err(e) => Err(format!("{}", e))
                }
            },
            Err(e) => Err(format!("{}", e))
        }
    }
}

fn load_flow_contents(flow: &Flow) {
    // Load subflows from FlorRefs
    //    pub flow: Option<Vec<FlowRef>>,
    // pub flows: Vec<Box<Flow>>

    // load entities from entity refs
    // pub entity: Option<Vec<EntityRef>>,
    // let entities: Vec<Entity> = Vec::new();

    // Create the IOs from IO Refs?
    // pub io: Option<Vec<IO>>,

    // Check the connections and connect them up with refs?
    //pub connection: Option<Vec<Connection>>,

    // Validate all is consistent now it's loaded??
    // flow.validate()
}

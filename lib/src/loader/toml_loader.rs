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
                    Ok(flow) => Ok(flow),
                    Err(e) => Err(format!("{}", e))
                }
            },
            Err(e) => Err(format!("{}", e))
        }
    }
}

/*
fn load_flow(source: PathBuf) -> Result {
    // TODO catch error
    let name: Name = yaml["context"].as_str().unwrap().to_string();

        // TODO check all tags present are allowed in a context

        let entities: Vec<Entity> = Vec::new();
        // 	entities = yaml["entities"]
        // create each entity
        // load it using .load()

        let values: Vec<Value> = vec![];
        // yaml["values"]

        let flows: Vec<(String, String, RefCell<Flow>)> = vec![];
        // flow = yaml["flow"]

        let connection_set: ConnectionSet = ConnectionSet::new(vec![], vec![]);

    // 	connection_set = yaml["connection_set"]
    //	let context = Context::new(name, path, entities, values, flows, connection_set);

    // TODO validate this context as loaded

    // TODO load the flow contained if there is one
        let sub_flow: &Yaml = &yaml["flow"];
        let flow_name: String = sub_flow["name"].as_str().unwrap().to_string();
        let flow = Some(Box::new(Flow{ name : flow_name}));

    // Then validate the conections between this context and the contained flow
    let context = Context::new(source, name, None);

    Result::Context(context)
}
*/

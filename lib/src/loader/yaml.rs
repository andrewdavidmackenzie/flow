extern crate yaml_rust;

use self::yaml_rust::{YamlLoader, Yaml};
use description::context::Context;
use description::flow::Flow;
use description::name::Name;

use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::io::prelude::*;

/* use description::flow::Flow;
use description::entity::Entity;
use description::io::IOSet;
use description::value::Value;
use description::function::Function;
use description::connection::ConnectionSet;*/

use loader::loader::Result;

const CONTEXT_TAGS: &'static [&'static str] = &["context", "entities", "values", "flow", "connection_set"];
const FLOW_TAGS: &'static [&'static str] = &["flow", "ioSet", "flows", "connection_set", "values", "functions"];

fn load_context(source: PathBuf, yaml: &Yaml) -> Result {
    // TODO catch error
    let name: Name = yaml["context"].as_str().unwrap().to_string();

    /*
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
    */

    // 	connection_set = yaml["connection_set"]
    //	let context = Context::new(name, path, entities, values, flows, connection_set);

    // TODO validate this context as loaded

    // TODO load the flow contained if there is one
/*
    let sub_flow: &Yaml = &yaml["flow"];
    let flow_name: String = sub_flow["name"].as_str().unwrap().to_string();
    let flow = Some(Box::new(Flow{ name : flow_name}));
*/

    // Then validate the conections between this context and the contained flow
    let context = Context::new(source, name, None);

    Result::Context(context)
}

fn load_flow(yaml: &Yaml) -> Result {
    let name: String = yaml["context"].as_str().unwrap().to_string();
/*
    let name: String = match yaml["flow"].as_str() {
		Some(el) => el.to_string(),
		None => Result::Error("Could not find flow name".to_string())
	};
*/

	// TODO check all tags present are allowed in a flow
/*
	// yaml["flows"]
	let flows: Vec<(String, String, Box<Flow>)> = vec![];

	// yaml["connection_set"] "connections"
	// yaml["connection_set"] "requests"
	let connection_set = ConnectionSet::new(vec![], vec![]);

	// yaml["ioSet"] "inputs"
	// yaml["ioSet"] "outputs"
	// yaml["ioSet"] "input_outputs"
	// yaml["ioSet"] "output_inputs"
	let ios: IOSet = IOSet::new(vec![], vec![], vec![], vec![]);

	// yaml["values"]
	let values: Vec<Value> = vec![];

	// yaml["functions"]
	let functions: Vec<Function> = vec![];

	let flow = Flow::new(name, path, flows, connection_set, ios, values, functions);
	*/

    let flow = Flow { name: name };
	Result::Flow(flow)
}

/*
 read the yaml file and parse the contents
 */
pub fn load(file_path: PathBuf) -> Result {
    let file = File::open(&file_path).unwrap(); // TODO handle error
    let mut buf_reader = BufReader::new(file);
    let mut contents = String::new();
    match buf_reader.read_to_string(&mut contents) {
        Ok(_) => {
            let docs = YamlLoader::load_from_str(&contents).unwrap();
            let doc = &docs[0];

            // TODO for now assume is a context - maybe load first element and decide based on that?
            load_context(file_path, doc)
        },
        Err(e) => Result::Error(format!("{}", e))
    }
}
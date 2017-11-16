extern crate yaml_rust;

use self::yaml_rust::{YamlLoader, Yaml};
use description::context::Context;

use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;

/* use description::flow::Flow;
use description::entity::Entity;
use description::io::IOSet;
use description::value::Value;
use description::function::Function;
use description::connection::ConnectionSet;*/

use parser::parser::Result;

const CONTEXT_TAGS: &'static [&'static str] = &["context", "entities", "values", "flows", "connection_set"];
const FLOW_TAGS: &'static [&'static str] = &["flow", "ioSet", "flows", "connection_set", "values", "functions"];


/*

validate model (see check)

load flow definition from file specified in arguments
    - load any referenced to included flows also

construct overall list of functions

construct list of connections

construct initial list of all functions able to produce output
    - start from external sources at level 0

do
    - identify all functions which receive input from active sources
    - execute all those functions
    - functions producing output added to list of active sources
while functions pending input

 */

fn parse_context(yaml: &Yaml) -> Result {
    // TODO catch error
    let name: String = yaml["context"].as_str().unwrap().to_string();

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

    let context = Context { name: name };

    Result::ContextLoaded(context)
}

/*
fn parse_flow(yaml: &Yaml, path: &str) -> Result {
	let name: String = match yaml["flow"].as_str() {
		Some(el) => el.to_string(),
		None => return parser::Result::Error("Could not find flow name".to_string()),
	};

	// TODO check all tags present are allowed in a flow

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
	Result::FlowLoaded(flow)
}

fn parse(yaml: &Yaml, path: &str, context_allowed: bool) -> Result {
	if !yaml["context"].as_str().unwrap().to_string().is_empty() {
		if !context_allowed {
			return parser::Result::Error("context: Not allowed at this point".to_string());
		}

		return parse_context(&yaml, path)
	}

	if !yaml["flow"].as_str().unwrap().to_string().is_empty() {
		return parse_flow(&yaml, path)
	}

	Result::Error("No 'context:' or 'flow:' was found".to_string())
}

*/

/*
 read the yaml file and parse the contents
 */
pub fn load(file: File) -> Result {
    let mut buf_reader = BufReader::new(file);
    let mut contents = String::new();
    match buf_reader.read_to_string(&mut contents) {
        Ok(_) => {
            let docs = YamlLoader::load_from_str(&contents).unwrap();
            let doc = &docs[0];

            // TODO for now assume is a context - maybe load first element and decide based on that?
            parse_context(doc)
        },
        Err(e) => Result::Error(format!("{}", e))
    }
}
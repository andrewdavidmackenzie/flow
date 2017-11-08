extern crate yaml_rust;

use std::error::Error;
use std::io::prelude::*;
use std::io;
use std::fs::File;
use std::fs;
use std::path::Path;
use std::cell::RefCell;

use self::yaml_rust::{YamlLoader, Yaml, scanner};

use description::context::Context;
use description::flow::Flow;
use description::entity::Entity;
use description::io::IOSet;
use description::value::Value;
use description::function::Function;
use description::connection::ConnectionSet;

use parser::parser;

const CONTEXT_TAGS: &'static [ &'static str ] = &["context", "entities", "values", "flows", "connection_set"];
const FLOW_TAGS: &'static [ &'static str ] = &["flow", "ioSet", "flows", "connection_set", "values", "functions"];

/*
fn parse_context(yaml: &Yaml, path: &str) -> parser::Result {
	// TODO catch error
	let name: String = yaml["context"].as_str().unwrap().to_string();

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
	let context = Context::new(name, path, entities, values, flows, connection_set);

	parser::Result::ContextLoaded(context)
}

fn parse_flow(yaml: &Yaml, path: &str) -> parser::Result {
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
	parser::Result::FlowLoaded(flow)
}

fn parse(yaml: &Yaml, path: &str, context_allowed: bool) -> parser::Result {
	if !yaml["context"].as_str().unwrap().to_string().is_empty() {
		if !context_allowed {
			return parser::Result::Error("context: Not allowed at this point".to_string());
		}

		return parse_context(&yaml, path)
	}

	if !yaml["flow"].as_str().unwrap().to_string().is_empty() {
		return parse_flow(&yaml, path)
	}

	parser::Result::Error("No 'context:' or 'flow:' was found".to_string())
}

fn read(path: &str) -> Result<Vec<Yaml>, String> {
	// Open the path in read-only mode, returns `io::Result<File>`
	let mut file = try!(File::open(path).map_err(|e| e.to_string()));

	let mut contents = String::new();
	try!(file.read_to_string(&mut contents).map_err(|e| e.to_string()));

	let docs = try!(YamlLoader::load_from_str(contents.as_ref()).map_err(|e| e.to_string()));

	Ok(docs)
}

*/

/*
 read the yaml file and parse the contents
 */
pub fn load(path: &str, context_allowed: bool) -> parser::Result {
	/*
	match read(path) {
		// YAML can have multiple files within one doc, we will just parse the first one found
		Ok(docs) => parse(&docs[0], path, context_allowed),
		Err(why) => parser::Result::Error(format!("Error reading yaml: {}", why.to_string())),
	}
	*/
	parser::Result::ContextLoaded(Context{}) // TODO re-enable
}
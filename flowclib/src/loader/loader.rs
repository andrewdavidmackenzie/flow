use std::path::PathBuf;

use model::flow::Flow;
use model::function::Function;
use loader::file_helper::get_contents;
use loader::file_helper::get_canonical_path;
use loader::loader_helper::get_loader;
use model::dumper;
use std::mem::replace;

// Any loader of a file extension have to implement these methods
pub trait Loader {
    fn load_flow(&self, contents: &str) -> Result<Flow, String>;
    fn load_function(&self, contents: &str) -> Result<Function, String>;
}

pub trait Validate {
    fn validate(&self) -> Result<(), String>;
}

/// # Example
/// ```
/// use std::path::PathBuf;
/// use flowclib::loader::loader;
///
/// let path = PathBuf::from("../samples/hello-world-simple/context.toml");
/// loader::load(path, false).unwrap();
/// ```
pub fn load(file_path: PathBuf, dump: bool) -> Result<Flow, String> {
    let flow = load_flow("", file_path);

    if let &Ok(ref loaded_flow) = &flow {
        if dump {
            dumper::dump(loaded_flow, 0);
        }
    }

    flow
}

fn load_flow(parent_route: &str, file_path: PathBuf) -> Result<Flow, String> {
    let mut flow = load_single_flow(parent_route, file_path)?;
    load_subflows(&mut flow)?;
    build_connections(&mut flow);
    Ok(flow)
}

/// # Example
/// ```
/// use std::path::PathBuf;
/// use flowclib::loader::loader;
///
/// let path = PathBuf::from("../samples/hello-world-simple/context.toml");
/// loader::load_single_flow("", path).unwrap();
/// ```
pub fn load_single_flow(parent_route: &str, file_path: PathBuf) -> Result<Flow, String> {
    let loader = get_loader(&file_path)?;
    let contents = get_contents(&file_path)?;
    let mut flow = loader.load_flow(&contents)?;
    flow.source = file_path;
    flow.route = format!("{}/{}", parent_route, flow.name);
    flow.validate()?;
    load_functions(&mut flow)?;
    load_values(&mut flow)?;
    flow.set_io_routes();
    Ok(flow)
}

/// # Example
/// ```
/// use std::path::PathBuf;
/// use flowclib::loader::loader;
///
/// let path = PathBuf::from("../samples/hello-world-simple/terminal.toml");
/// loader::load_function(&path, "").unwrap();
/// ```
pub fn load_function(file_path: &PathBuf, parent_name: &str) -> Result<Function, String> {
    let loader = get_loader(file_path)?;
    let contents = get_contents(file_path)?;
    let mut function = loader.load_function(&contents)?;
    function.route = format!("{}/{}", parent_name, function.name);

    if let Some(ref mut inputs) = function.inputs {
        for ref mut input in inputs {
            input.route = format!("{}/{}", function.route, input.name);
        }
    }
    if let Some(ref mut outputs) = function.outputs {
        for ref mut output in outputs {
            output.route = format!("{}/{}", function.route, output.name);
        }
    }
    function.validate()?;
    Ok(function)
}

/*
    Load all functions referenced from a flow
*/
fn load_functions(flow: &mut Flow) -> Result<(), String> {
    if let Some(ref mut function_refs) = flow.function_refs {
        for ref mut function_ref in function_refs {
            let function_path = get_canonical_path(PathBuf::from(&flow.source),
                                                   PathBuf::from(&function_ref.source));
            function_ref.function = load_function(&function_path, &flow.route)?;
        }
    }
    Ok(())
}

/*
    Load all values defined in a flow
*/
fn load_values(flow: &mut Flow) -> Result<(), String> {
    if let Some(ref mut values) = flow.values {
        for ref mut value in values {
            value.route = format!("{}/{}", flow.route, value.name);
        }
    }
    Ok(())
}

/*
    Load all subflows referenced from a flow
*/
fn load_subflows(flow: &mut Flow) -> Result<(), String> {
    // Load subflows from References
    if let Some(ref mut flow_refs) = flow.flow_refs {
        for ref mut flow_ref in flow_refs {
            let subflow_path = get_canonical_path(PathBuf::from(&flow.source),
                                                  PathBuf::from(&flow_ref.source));
            let subflow = load_flow(&flow.route, subflow_path)?;
            flow_ref.flow = subflow;
        }
    }
    Ok(())
}

/*
    Change the names of connections to be routes to the alias used in this flow,
    in the process ensuring they exist, that direction is correct and types match

    Connection to/from Formats:
        "value/message"
        "input/input_name"
        "output/output_name"

        "flow/flow_name/io_name"
        "function/function_name/io_name"
*/
fn build_connections(flow: &mut Flow) {
    if flow.connections.is_none() { return; }

    // get connections out of self - so we can use immutable references to self inside loop
    let connections = replace(&mut flow.connections, None);
    let mut connections = connections.unwrap();

    for connection in connections.iter_mut() {
        // TODO eliminate output as a possible source
        if let Ok((from_route, from_type, starts_at_flow)) = flow.get_route_and_type(&connection.from) {
            // TODO eliminate to as a possible source
            if let Ok((to_route, to_type, ends_at_flow)) = flow.get_route_and_type(&connection.to) {
                if from_type == to_type {
                    connection.from_route = from_route;
                    connection.starts_at_flow = starts_at_flow;
                    connection.to_route = to_route;
                    connection.ends_at_flow = ends_at_flow;
                } else {
                    eprintln!("Type mismatch from '{}' of type '{}' to '{}' of type '{}'",
                             from_route, from_type, to_route, to_type);
                }
            } else {
                eprintln!("Did not find destination: {}", connection.to);
            }
        } else {
            eprintln!("Did not find source: {}", connection.from);
        }
    }

    // put connections back into self
    replace(&mut flow.connections, Some(connections));
}

#[test]
#[ignore]
fn sample_hello_world_simple_yaml() {
    let path = PathBuf::from("../samples/hello-world-simple-yaml/context.yaml");
    load(path, false).unwrap();
}

#[test]
#[ignore]
fn sample_hello_world_yaml() {
    let path = PathBuf::from("../samples/hello-world-yaml/context.yaml");
    load(path, false).unwrap();
}

#[test]
fn dump_hello_world_simple() {
    let path = PathBuf::from("../samples/hello-world-simple/context.toml");
    load(path, true).unwrap();
}

#[test]
fn dump_hello_world_context() {
    let path = PathBuf::from("../samples/hello-world/context.toml");
    load(path, true).unwrap();
}

#[test]
fn dump_hello_world_include() {
    let path = PathBuf::from("../samples/hello-world-include/context.toml");
    load(path, true).unwrap();
}

#[test]
fn dump_hello_world_flow1() {
    let path = PathBuf::from("../samples/hello-world/flow1.toml");
    load(path, true).unwrap();
}

#[test]
fn dump_complex1() {
    let path = PathBuf::from("../samples/complex1/context.toml");
    load(path, true).unwrap();
}

#[test]
fn dump_fibonacci() {
    let path = PathBuf::from("../samples/fibonacci/context.toml");
    load(path, true).unwrap();
}
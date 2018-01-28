use model::flow::Flow;
use model::function::Function;
use content::provider;
use loader::loader_helper::get_loader;
use model::dumper;
use std::mem::replace;

use url::Url;

// TODO use when we extend beyond just files -
// These are the schemes we will accept for references to flows/functions
// const SCHEMES: [&'static str; 4]= ["file:", "http:", "https:", "lib:"];

// Any loader of a file type has to implement these methods
pub trait Loader {
    fn load_flow(&self, contents: &str) -> Result<Flow, String>;
    fn load_function(&self, contents: &str) -> Result<Function, String>;
}

pub trait Validate {
    fn validate(&self) -> Result<(), String>;
}

/// load a flow definition from the `file_path` specified, and optionally dump a representation
/// of the flow to stdout using the `dump` boolean.
///
/// It recursively loads all flows that are referenced.
///
/// The return value is a `Result` containing the hierarchical `Flow` in memory, or a `String`
/// describing the error found while loading.
///
/// # Example
/// ```
/// extern crate url;
/// extern crate flowclib;
/// use std::env;
///
/// let mut url = url::Url::from_file_path(env::current_dir().unwrap()).unwrap();
/// url = url.join("samples/hello-world-simple/context.toml").unwrap();
/// flowclib::loader::loader::load(&url, false).unwrap();
/// ```
pub fn load(url: &Url, dump: bool) -> Result<Flow, String> {
    let flow = load_flow("", url);

    if let &Ok(ref loaded_flow) = &flow {
        if dump {
            dumper::dump(loaded_flow, 0);
        }
    }

    flow
}

fn load_flow(parent_route: &str, url: &Url) -> Result<Flow, String> {
    let mut flow = load_single_flow(parent_route, url)?;
    load_subflows(&mut flow)?;
    build_connections(&mut flow);
    Ok(flow)
}

/// load a flow definition from the `file_path` specified, and optionally dump a representation
/// of the flow to stdout using the `dump` boolean.
///
/// It loads only the flow defined in the file specified and does not recursively loads all
/// flows that are referenced.
///
/// The return value is a `Result` containing the hierarchical `Flow` in memory, or a `String`
/// describing the error found while loading.
///
/// # Example
/// ```
/// extern crate url;
/// extern crate flowclib;
///
/// use std::env;
///
/// let mut url = url::Url::from_file_path(env::current_dir().unwrap()).unwrap();
/// url = url.join("samples/hello-world-simple/context.toml").unwrap();
/// flowclib::loader::loader::load_single_flow("", &url).unwrap();
/// ```
pub fn load_single_flow(parent_route: &str, url: &Url) -> Result<Flow, String> {
    let loader = get_loader(url)?;
    let contents = provider::get_contents(url)?;
    let mut flow = loader.load_flow(&contents)?;
    flow.source_url = url.clone();
    flow.route = format!("{}/{}", parent_route, flow.name);
    flow.validate()?;
    load_functions(&mut flow)?;
    load_values(&mut flow)?;
    flow.set_io_routes();
    Ok(flow)
}

/// load a function definition from the `file_path` specified, the `parent_route` parameter
/// specifies where in the flow hierarchiy this instance of the function is referenced, and is
/// used to create routes to the functions inputs and outputs.
///
/// # Example
/// ```
/// extern crate url;
/// extern crate flowclib;
/// use std::env;
///
/// let mut url = url::Url::from_file_path(env::current_dir().unwrap()).unwrap();
/// url = url.join("samples/hello-world-simple/stdout.toml").unwrap();
/// flowclib::loader::loader::load_function(&url, "/root_flow").unwrap();
/// ```
pub fn load_function(url: &Url, parent_route: &str) -> Result<Function, String> {
    let loader = get_loader(url)?;
    let contents = provider::get_contents(url)?;
    let mut function = loader.load_function(&contents)?;
    function.route = format!("{}/{}", parent_route, function.name);

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
            let function_url = flow.source_url.join(&function_ref.source).expect("URL join error");
            function_ref.source_url = function_url.clone();
            function_ref.function = load_function(&function_url, &flow.route)?;
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
            let subflow_url = flow.source_url.join(&flow_ref.source).expect("URL join error");
            let subflow = load_flow(&flow.route, &subflow_url)?;
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
                    error!("Type mismatch from '{}' of type '{}' to '{}' of type '{}'",
                           from_route, from_type, to_route, to_type);
                }
            } else {
                error!("Did not find destination: {}", connection.to);
            }
        } else {
            error!("Did not find source: {}", connection.from);
        }
    }

    // put connections back into self
    replace(&mut flow.connections, Some(connections));
}


#[cfg(test)]
mod test {
    use url::Url;
    use std::env;
    use super::load;

    fn url_from_rel_path(path: &str) -> Url {
        let parent = Url::from_file_path(env::current_dir().unwrap()).unwrap();
        parent.join(path).unwrap()
    }

    #[test]
    #[ignore]
    fn sample_hello_world_simple_yaml() {
        load(&url_from_rel_path("samples/hello-world-simple-yaml/context.yaml"), false).unwrap();
    }

    #[test]
    #[ignore]
    fn sample_hello_world_yaml() {
        load(&url_from_rel_path("samples/hello-world-yaml/context.yaml"), false).unwrap();
    }

    #[test]
    fn dump_hello_world_simple() {
        load(&url_from_rel_path("samples/hello-world-simple/context.toml"), true).unwrap();
    }

    #[test]
    fn dump_hello_world_context() {
        load(&url_from_rel_path("samples/hello-world/context.toml"), true).unwrap();
    }

    #[test]
    fn dump_hello_world_include() {
        load(&url_from_rel_path("samples/hello-world-include/context.toml"), true).unwrap();
    }

    #[test]
    fn dump_hello_world_flow1() {
        load(&url_from_rel_path("samples/hello-world/flow1.toml"), true).unwrap();
    }

    #[test]
    fn dump_complex1() {
        load(&url_from_rel_path("samples/complex1/context.toml"), true).unwrap();
    }

    #[test]
    fn dump_fibonacci() {
        load(&url_from_rel_path("samples/fibonacci/context.toml"), true).unwrap();
    }
}
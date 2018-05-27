use model::flow::Flow;
use model::function::Function;
use model::connection::Direction::FROM;
use model::connection::Direction::TO;
use model::name::Name;
use model::name::HasName;
use model::route::Route;
use model::route::HasRoute;
use model::route::SetRoute;
use content::provider;
use loader::loader_helper::get_loader;
use std::mem::replace;

use url::Url;

// Any loader has to implement these methods
pub trait Loader {
    fn load_flow(&self, contents: &str) -> Result<Flow, String>;
    fn load_function(&self, contents: &str) -> Result<Function, String>;
}

pub trait Validate {
    fn validate(&self) -> Result<(), String>;
}

/// load a flow definition from the `file_path` specified
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
/// flowclib::loader::loader::load(&"root".to_string(), &url).unwrap();
/// ```
pub fn load(alias: &Name, url: &Url) -> Result<Flow, String> {
    load_flow(&Route::from(""), alias, url)
        .map_err(|e| format!("while loading flow from Url '{}'\n\t- {}", url, e.to_string()))
}

fn load_flow(parent_route: &Route, alias: &Name, url: &Url) -> Result<Flow, String> {
    let mut flow = load_single_flow(parent_route, alias, url)?;
    load_subflows(&mut flow)?;
    build_flow_connections(&mut flow)?;
    Ok(flow)
}

/// load a flow definition from the `file_path` specified
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
/// flowclib::loader::loader::load_single_flow(&flowclib::model::route::Route::from("root_flow"),
///                                            &flowclib::model::name::Name::from("call-me-hello"),
///                                            &url).unwrap();
/// ```
pub fn load_single_flow(parent_route: &Route, alias: &Name, url: &Url) -> Result<Flow, String> {
    let (resolved_url, lib_ref) = provider::resolve(url)?;
    let loader = get_loader(&resolved_url)?;
    info!("Loading flow from '{}'", resolved_url);
    let contents = provider::get(&resolved_url)?;
    let mut flow = loader.load_flow(&contents)
        .map_err(|e| format!("while loading flow - {}", e.to_string()))?;
    flow.alias = alias.clone();
    flow.source_url = resolved_url;
    flow.set_route_from_parent(parent_route);
    if let Some(lr) = lib_ref {
        flow.lib_references.push(lr);
    };
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
/// url = url.join("samples/reverse-echo/reverse.toml").unwrap();
/// flowclib::loader::loader::load_function(&url,
///                                         &flowclib::model::route::Route::from("/root_flow"),
///                                         &flowclib::model::name::Name::from("call-me-hello")).unwrap();
/// ```
pub fn load_function(url: &Url, parent_route: &Route, alias: &Name) -> Result<Function, String> {
    debug!("Loading function from '{}'", url);
    let (resolved_url, lib_ref) = provider::resolve(url)?;
    let loader = get_loader(&resolved_url)?;
    let contents = provider::get(&resolved_url)?;
    let mut function = loader.load_function(&contents)?;
    function.set_alias(alias.to_string());
    function.set_source_url(resolved_url.clone());
    function.set_lib_reference(lib_ref);
    function.set_routes_from_parent(parent_route);
    function.validate()?;
    Ok(function)
}

/*
    Load all functions referenced from a flow
*/
fn load_functions(flow: &mut Flow) -> Result<(), String> {
    let parent_route = &flow.route().clone();
    if let Some(ref mut function_refs) = flow.function_refs {
        debug!("Loading functions for flow '{}'", flow.source_url);
        for ref mut function_ref in function_refs {
            let function_url = flow.source_url.join(&function_ref.source)
                .map_err(|_e| "URL join error")?;
            function_ref.function = load_function(&function_url, parent_route, &function_ref.alias())
                .map_err(|e| format!("while loading function from Url '{}' - {}",
                                     function_url, e.to_string()))?;
            if let &Some(ref lib_ref) = function_ref.function.get_lib_reference() {
                flow.lib_references.push(format!("{}/{}", lib_ref, function_ref.function.name()));
            }
        }
    }
    Ok(())
}

/*
    Load all values defined in a flow
*/
fn load_values(flow: &mut Flow) -> Result<(), String> {
    let parent_route = &flow.route().clone();
    if let Some(ref mut values) = flow.values {
        debug!("Loading values for flow '{}'", flow.source_url);
        for ref mut value in values {
            value.set_routes_from_parent(parent_route);
        }
    }
    Ok(())
}

/*
    Load all sub-flows referenced from a flow via the flow_references
*/
fn load_subflows(flow: &mut Flow) -> Result<(), String> {
    let parent_route = &flow.route().clone();
    if let Some(ref mut flow_refs) = flow.flow_refs {
        debug!("Loading sub-flows of flow '{}'", flow.source_url);
        for ref mut flow_ref in flow_refs {
            let subflow_url = flow.source_url.join(&flow_ref.source).expect("URL join error");
            let subflow = load_flow(parent_route, &flow_ref.alias(), &subflow_url)?;
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
fn build_flow_connections(flow: &mut Flow) -> Result<(), String> {
    if flow.connections.is_none() { return Ok(()); }

    debug!("Building connections for flow '{}'", flow.source_url);

    let mut error_count = 0;

    // get connections out of self - so we can use immutable references to self inside loop
    let connections = replace(&mut flow.connections, None);
    let mut connections = connections.unwrap();

    for connection in connections.iter_mut() {
        connection.check_for_loops(flow.source_url.as_str())?;
        match flow.get_route_and_type(FROM, &connection.from) {
            Ok(from) => {
                debug!("Found source of connection:\n{:#?}", from);
                match flow.get_route_and_type(TO, &connection.to) {
                    Ok(to) => {
                        debug!("Found destination of connection:\n{:#?}", to);
                        if (from.datatype(0) == to.datatype(0)) ||
                            from.datatype(0) == "Json" || to.datatype(0) == "Json" {
                            debug!("Connection source and destination types match, connection built");
                            connection.from_io = from;
                            connection.to_io = to;
                        } else {
                            error!("Type mismatch in flow '{}' connection:\n\nfrom\n\n{:#?}\n\nto\n\n{:#?}",
                                   flow.source_url, from, to);
                            error_count += 1;
                        }
                    }
                    Err(error) => {
                        error!("Did not find connection destination: '{}' specified in flow '{}'\n\t\t{}",
                               connection.to, flow.source_url, error);
                        error_count += 1;
                    }
                }
            }
            Err(error) => {
                error!("Did not find connection source: '{}' specified in flow '{}'\n\t\t{}",
                       connection.from, flow.source_url, error);
                error_count += 1;
            }
        }
    }

    // put connections back into self
    replace(&mut flow.connections, Some(connections));

    if error_count == 0 {
        debug!("All connections built inside flow '{}'", flow.source_url);
        Ok(())
    } else {
        Err(format!("{} connections errors found in flow '{}'", error_count, flow.source_url))
    }
}
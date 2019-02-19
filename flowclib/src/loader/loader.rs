use model::flow::Flow;
use model::function::Function;
use model::process::Process;
use model::connection::Direction::FROM;
use model::connection::Direction::TO;
use model::name::Name;
use model::name::HasName;
use model::route::Route;
use model::route::HasRoute;
use model::route::SetRoute;
use loader::loader_helper::get_loader;
use flowrlib::provider::Provider;
use model::process::Process::FlowProcess;
use model::process::Process::FunctionProcess;
use std::mem::replace;
use flowrlib::url;

// Any loader has to implement these methods
pub trait Loader {
    fn load_process(&self, contents: &str) -> Result<Process, String>;
}

pub trait Validate {
    fn validate(&self) -> Result<(), String>;
}

/// load a process definition from `url`, recursively loading all sub-processes referenced.
///
/// The return value is a `Result` containing the `Process`, or a `String`
/// describing the error found while loading.
///
/// # Example
/// ```
/// extern crate url;
/// extern crate flowclib;
/// extern crate flowrlib;
///
/// use flowrlib::provider::Provider;
/// use std::env;
/// use url::Url;
///
/// // Clients need to provide a Provider of content for the loader
/// struct DummyProvider {};
///
/// impl Provider for DummyProvider {
///     fn resolve(&self, url: &str, default_filename: &str) -> Result<(String, Option<String>), String> {
///        // Just fake the url resolution in this example
///        Ok((url.to_string(), None))
///     }
///
///    fn get(&self, url: &str) -> Result<Vec<u8>, String> {
///        // Return the simplest flow definition possible
///        Ok("flow = \"test\"".as_bytes().to_owned())
///     }
/// }
///
/// let parent_route = "".to_string();
/// let alias = "my process".to_string();
/// let dummy_provider = DummyProvider {};
/// let mut url = url::Url::from_file_path(env::current_dir().unwrap()).unwrap();
/// url = url.join("samples/hello-world-simple/context.toml").unwrap();
/// flowclib::loader::loader::load_process(&parent_route, &alias, &url.to_string(), &dummy_provider).unwrap();
/// ```
pub fn load_process(parent_route: &Route, alias: &Name, url: &str, provider: &Provider) -> Result<Process, String> {
    let (resolved_url, lib_ref) = provider.resolve(url, "context.toml")?;
    let loader = get_loader(&resolved_url)?;
    info!("Loading process with alias = '{}' from url = '{}' ", alias, resolved_url);
    let contents = provider.get(&resolved_url)?;

    let mut process = loader.load_process(&String::from_utf8(contents).unwrap())?;

    match process {
        FlowProcess(ref mut flow) => {
            config_flow(flow, &resolved_url, parent_route, alias)?;
            load_values(flow)?;
            load_subprocesses(flow, provider)?;
            build_flow_connections(flow)?;
        }
        FunctionProcess(ref mut function) => {
            config_function(function, &resolved_url, parent_route, alias, lib_ref)?;
        }
    }

    Ok(process)
}

/*
    Load all sub-processes referenced from a flow via the process_reference fields
*/
fn load_subprocesses(flow: &mut Flow, provider: &Provider) -> Result<(), String> {
    if let Some(ref mut process_refs) = flow.process_refs {
        for process_ref in process_refs {
            let subprocess_url = url::join(&flow.source_url, &process_ref.source);
            process_ref.process = load_process(&flow.route, &process_ref.alias(), &subprocess_url, provider)?;

            if let FunctionProcess(ref function) = process_ref.process {
                if let Some(lib_ref) = function.get_lib_reference() {
                    flow.lib_references.push(format!("{}/{}", lib_ref, function.name()));
                }
            }
        }
    }
    Ok(())
}

fn config_function(function: &mut Function, source_url: &str, parent_route: &Route, alias: &Name,
                   lib_ref: Option<String>) -> Result<(), String> {
    function.set_alias(alias.to_string());
    function.set_source_url(source_url.clone());
    function.set_lib_reference(lib_ref);
    function.set_routes_from_parent(parent_route, false);
    function.validate()
}

fn config_flow(flow: &mut Flow, source_url: &str, parent_route: &Route, alias: &Name)
    -> Result<(), String> {
    flow.alias = alias.to_string();
    flow.source_url = source_url.to_string();
    flow.set_routes_from_parent(parent_route, true);
    flow.validate()
}

/*
    Load all the values that are defined in a flow
*/
fn load_values(flow: &mut Flow) -> Result<(), String> {
    let parent_route = &flow.route().clone();
    if let Some(ref mut values) = flow.values {
        debug!("Loading values for flow '{}'", flow.source_url);
        for ref mut value in values {
            value.set_routes_from_parent(parent_route, false);
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

        "process/flow_name/io_name"
        "process/function_name/io_name"
*/
fn build_flow_connections(flow: &mut Flow) -> Result<(), String> {
    if flow.connections.is_none() { return Ok(()); }

    debug!("Building connections for flow '{}'", flow.source_url);

    let mut error_count = 0;

    // get connections out of self - so we can use immutable references to self inside loop
    let connections = replace(&mut flow.connections, None);
    let mut connections = connections.unwrap();

/*
TODO when loading a flow we need to do this check for connections within the flow
- needs connections to all inputs or can't run
- output should be connected also
*/


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
        debug!("All connections inside flow '{}' successfully built", flow.source_url);
        Ok(())
    } else {
        Err(format!("{} connections errors found in flow '{}'", error_count, flow.source_url))
    }
}
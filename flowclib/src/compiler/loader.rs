use std::collections::HashMap;

use deserializers::deserializer_helper::get_deserializer;
use flowrlib::input::InputInitializer;
use flowrlib::provider::Provider;
use flowrlib::url;
use model::flow::Flow;
use model::function::Function;
use model::io::IO;
use model::name::HasName;
use model::process::Process;
use model::process::Process::FlowProcess;
use model::process::Process::FunctionProcess;
use model::route::Route;
use model::route::SetRoute;

// Any deserializer has to implement this method
pub trait Deserializer {
    fn deserialize(&self, contents: &str) -> Result<Process, String>;
}

pub trait Validate {
    fn validate(&self) -> Result<(), String>;
}

/// Load a context process definition from `url`, recursively loading all sub-processes referenced.
///
/// The return value is a `Result` containing the `Process`, or a `String` describing the error
/// found while loading.
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
/// // Clients need to provide a Provider of content for the loader as flowlibc is independant of
/// // file systems and io.
/// struct DummyProvider;
///
/// // A Provider must implement the `Provider` trait, with the methods to `resolve` a URL and to
/// // `get` the contents for parsing.
/// impl Provider for DummyProvider {
///     fn resolve(&self, url: &str, default_filename: &str) -> Result<(String, Option<String>), String> {
///        // Just fake the url resolution in this example
///        Ok((url.to_string(), None))
///     }
///
///    fn get(&self, url: &str) -> Result<Vec<u8>, String> {
///        // Return the simplest flow definition possible - ignoring the url passed in
///        Ok("flow = \"test\"".as_bytes().to_owned())
///     }
/// }
///
/// // Create an instance of the `DummyProvider`
/// let dummy_provider = DummyProvider{};
///
/// // load the flow from `url = file:///example.toml` using the `dummy_provider`
/// flowclib::compiler::loader::load_context("file:///example.toml", &dummy_provider).unwrap();
/// ```
pub fn load_context(url: &str, provider: &Provider) -> Result<Process, String> {
    load_process("", "context", url, provider, &None)
}

fn load_process(parent_route: &str, alias: &str, url: &str, provider: &Provider,
                initializations: &Option<HashMap<String, InputInitializer>>) -> Result<Process, String> {
    let (resolved_url, lib_ref) = provider.resolve(url, "context.toml")?;
    let contents = provider.get(&resolved_url)?;

    let deserializer = get_deserializer(&resolved_url)?;
    info!("Deserializing process with alias = '{}' from url = '{}' ", alias, resolved_url);
    let mut process = deserializer.deserialize(&String::from_utf8(contents).unwrap())?;

    match process {
        FlowProcess(ref mut flow) => {
            config_flow(flow, &resolved_url, &String::from(parent_route), alias, initializations)?;
            load_subprocesses(flow, provider)?;
            flow.build_connections()?;
        }
        FunctionProcess(ref mut function) => {
            config_function(function, &resolved_url, &String::from(parent_route), alias, lib_ref,
                            initializations)?;
        }
    }

    Ok(process)
}

/*
    Load all sub-processes referenced from a flow via the process_refs field
*/
fn load_subprocesses(flow: &mut Flow, provider: &Provider) -> Result<(), String> {
    if let Some(ref mut process_refs) = flow.process_refs {
        for process_ref in process_refs {
            let subprocess_url = url::join(&flow.source_url, &process_ref.source);
            process_ref.process = load_process(&flow.route, &process_ref.alias(),
                                               &subprocess_url, provider, &process_ref.initializations)?;

            if let FunctionProcess(ref mut function) = process_ref.process {
                if let Some(lib_ref) = function.get_lib_reference() {
                    flow.lib_references.push(format!("{}/{}", lib_ref, function.name()));
                }
            }
        }
    }
    Ok(())
}

fn config_function(function: &mut Function, source_url: &str, parent_route: &Route, alias: &str,
                   lib_ref: Option<String>, initializations: &Option<HashMap<String, InputInitializer>>)
                   -> Result<(), String> {
    function.set_alias(alias.to_string());
    function.set_source_url(source_url.clone());
    function.set_lib_reference(lib_ref);
    function.set_routes_from_parent(parent_route);
    IO::set_initial_values(&mut function.inputs, initializations);
    function.validate()
}

fn config_flow(flow: &mut Flow, source_url: &str, parent_route: &Route, alias: &str,
               initializations: &Option<HashMap<String, InputInitializer>>)
               -> Result<(), String> {
    flow.alias = alias.to_string();
    flow.source_url = source_url.to_string();
    IO::set_initial_values(flow.inputs_mut(), initializations);
    flow.set_routes_from_parent(parent_route);
    flow.validate()
}
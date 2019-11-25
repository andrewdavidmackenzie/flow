use std::collections::HashMap;

use flowrlib::input::InputInitializer;
use flowrlib::provider::Provider;
use flowrlib::url;
use log::{debug, info};

use crate::deserializers::deserializer_helper::get_deserializer;
use crate::errors::*;
use crate::model::flow::Flow;
use crate::model::function::Function;
use crate::model::io::IO;
use crate::model::library::Library;
use crate::model::name::HasName;
use crate::model::name::Name;
use crate::model::process::Process;
use crate::model::process::Process::FlowProcess;
use crate::model::process::Process::FunctionProcess;
use crate::model::route::Route;
use crate::model::route::SetRoute;

// All deserializers have to implement this method
pub trait Deserializer {
    fn deserialize(&self, contents: &str, url: Option<&str>) -> Result<Process>;
    fn name(&self) -> &'static str;
}

pub trait Validate {
    fn validate(&self) -> Result<()>;
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
/// use flowrlib::errors::*;
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
///     fn resolve_url(&self, url: &str, default_filename: &str, _ext: &[&str]) -> Result<(String, Option<String>)> {
///        // Just fake the url resolution in this example
///        Ok((url.to_string(), None))
///     }
///
///    fn get_contents(&self, url: &str) -> Result<Vec<u8>> {
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
pub fn load_context(url: &str, provider: &dyn Provider) -> Result<Process> {
    load_process(&Route::from(""), &Name::from("context"), url, provider, &None)
}

fn load_process(parent_route: &Route, alias: &Name, url: &str, provider: &dyn Provider,
                initializations: &Option<HashMap<String, InputInitializer>>) -> Result<Process> {
    let (resolved_url, lib_ref) = provider.resolve_url(url, "context", &["toml"])
        .chain_err(|| format!("Could not resolve the url: '{}'", url))?;
    debug!("Source URL '{}' resolved to: '{}'", url, resolved_url);
    let contents = provider.get_contents(&resolved_url)
        .chain_err(|| format!("Could not get contents of resolved url: '{}'", resolved_url))?;

    let deserializer = get_deserializer(&resolved_url)?;
    info!("Loading process with alias = '{}'", alias);
    debug!("Loading from url = '{}' with deserializer: '{}'", resolved_url, deserializer.name());
    let mut process = deserializer.deserialize(&String::from_utf8(contents).unwrap(),
                                               Some(url))
        .chain_err(|| format!("Could not deserialize process from content in '{}'", url))?;

    debug!("Deserialized flow, now parsing and loading any sub-processes");
    match process {
        FlowProcess(ref mut flow) => {
            config_flow(flow, &resolved_url, parent_route, alias, initializations)?;
            load_subprocesses(flow, provider)?;
            flow.build_connections()?;
        }
        FunctionProcess(ref mut function) => {
            config_function(function, &resolved_url, parent_route, alias, lib_ref,
                            initializations)?;
        }
    }

    Ok(process)
}

pub fn load_library(url: &str, provider: &dyn Provider) -> Result<Library> {
    let (resolved_url, _) = provider.resolve_url(url, "Library", &["toml"])
        .chain_err(|| format!("Could not resolve the url: '{}'", url))?;
    debug!("Source URL '{}' resolved to: '{}'", url, resolved_url);
    let contents = provider.get_contents(&resolved_url)
        .chain_err(|| format!("Could not get contents of resolved url: '{}'", resolved_url))?;

    toml::from_str(&String::from_utf8(contents).unwrap())
        .chain_err(|| format!("Error deserializing Toml from: '{:?}'", resolved_url))
}

/*
    Load all sub-processes referenced from a flow via the process_refs field
*/
fn load_subprocesses(flow: &mut Flow, provider: &dyn Provider) -> Result<()> {
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

fn config_function(function: &mut Function, implementation_url: &str, parent_route: &Route, alias: &Name,
                   lib_ref: Option<String>, initializations: &Option<HashMap<String, InputInitializer>>)
                   -> Result<()> {
    function.set_alias(alias);
    function.set_implementation_url(implementation_url.clone());
    function.set_lib_reference(lib_ref);
    function.set_routes_from_parent(parent_route);
    IO::set_initial_values(&mut function.inputs, initializations);
    function.validate()
}

fn config_flow(flow: &mut Flow, source_url: &str, parent_route: &Route, alias: &Name,
               initializations: &Option<HashMap<String, InputInitializer>>) -> Result<()> {
    flow.alias = alias.clone();
    flow.source_url = source_url.to_string();
    IO::set_initial_values(flow.inputs_mut(), initializations);
    flow.set_routes_from_parent(parent_route);
    flow.validate()
}

#[cfg(test)]
mod test {
    use toml;

    use crate::model::library::Library;

    #[test]
    fn deserialize_library() {
        let contents= include_str!("../../test_libs/Library_test.toml");
        let _: Library = toml::from_str(contents).unwrap();
    }
}
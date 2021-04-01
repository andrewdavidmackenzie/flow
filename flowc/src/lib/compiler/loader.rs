use std::collections::HashMap;

use log::{debug, info, trace};
use url::Url;

use flowrstructs::input::InputInitializer;
use flowrstructs::manifest::{Cargo, MetaData};
use provider::lib_provider::LibProvider;

use crate::deserializers::deserializer_helper::get_deserializer;
use crate::errors::*;
use crate::model::flow::Flow;
use crate::model::function::Function;
use crate::model::io::IO;
use crate::model::name::HasName;
use crate::model::name::Name;
use crate::model::process::Process;
use crate::model::process::Process::FlowProcess;
use crate::model::process::Process::FunctionProcess;
use crate::model::route::Route;
use crate::model::route::SetRoute;

/// All deserializers have to implement this trait for content deserialization, plus a method
/// to return their name to be able to inform the user of which deserializer was used
pub trait Deserializer {
    fn deserialize(&self, contents: &str, url: Option<&Url>) -> Result<Process>;
    fn name(&self) -> &'static str;
}

/// Many structs in the model implement the `Validate` method which is used to check the
/// description deserialized from file obeys some additional constraints that cannot be expressed
/// in the struct definition in `serde`
pub trait Validate {
    fn validate(&self) -> Result<()>;
}

/// Load a `Flow` definition from a `Url`, recursively loading all sub-processes referenced.
///
/// The return is a `Result` containing the `Process`, or a `String` describing the error
/// found while loading.
///
/// # Example
/// ```
/// use provider::lib_provider::LibProvider;
/// use provider::errors::Result;
/// use std::env;
/// use url::Url;
///
/// // Clients need to provide a Provider of content for the loader as flowlibc is independent of
/// // file systems and io.
/// struct DummyProvider;
///
/// // A Provider must implement the `Provider` trait, with the methods to `resolve` a URL and to
/// // `get` the contents for parsing.
/// impl LibProvider for DummyProvider {
///     fn resolve_url(&self, url: &Url, default_filename: &str, _ext: &[&str]) -> Result<(Url, Option<String>)> {
///        // Just fake the url resolution in this example
///        Ok((url.clone(), None))
///     }
///
///    fn get_contents(&self, url: &Url) -> Result<Vec<u8>> {
///        // Return the simplest flow definition possible - ignoring the url passed in
///        Ok("flow = \"test\"".as_bytes().to_owned())
///     }
/// }
///
/// // Create an instance of the `DummyProvider`
/// let dummy_provider = DummyProvider{};
///
/// // load the flow from `url = file:///example.toml` using the `dummy_provider`
/// flowclib::compiler::loader::load(&Url::parse("file:///example.toml").unwrap(), &dummy_provider).unwrap();
/// ```
pub fn load(url: &Url, provider: &dyn LibProvider) -> Result<Process> {
    trace!("load()");
    load_process(
        &Route::default(),
        &Name::default(),
        0,
        &mut 0,
        url,
        provider,
        &&None,
    )
}

#[allow(clippy::too_many_arguments)]
fn load_process(
    parent_route: &Route,
    alias: &Name,
    parent_flow_id: usize,
    flow_count: &mut usize,
    url: &Url,
    provider: &dyn LibProvider,
    initializations: &Option<HashMap<String, InputInitializer>>,
) -> Result<Process> {
    trace!("load_process()");
    trace!("  --> resolve_url()");
    let (resolved_url, lib_ref) = provider
        .resolve_url(url, "context", &["toml"])
        .chain_err(|| format!("Could not resolve the url: '{}'", url))?;
    debug!("Source URL '{}' resolved to: '{}'", url, resolved_url);
    trace!("  --> get_contents()");
    let contents = provider
        .get_contents(&resolved_url)
        .chain_err(|| format!("Could not get contents of resolved url: '{}'", resolved_url))?;

    trace!("  --> get_deserializer()");
    let deserializer = get_deserializer(&resolved_url)?;
    if !alias.is_empty() {
        info!("Loading process with alias = '{}'", alias);
    }

    debug!(
        "Loading process from url = '{}' with deserializer: '{}'",
        resolved_url,
        deserializer.name()
    );
    trace!("  --> deserialize()");
    let mut process = deserializer
        .deserialize(
            &String::from_utf8(contents).chain_err(|| "Could not read UTF8 contents")?,
            Some(url),
        )
        .chain_err(|| format!("Could not deserialize process from content in '{}'", url))?;

    debug!("Deserialized the flow, now parsing and loading any sub-processes");
    match process {
        FlowProcess(ref mut flow) => {
            config_flow(
                flow,
                &resolved_url,
                parent_route,
                alias,
                *flow_count,
                initializations,
            )?;
            *flow_count += 1;
            load_process_refs(flow, flow_count, provider)?;
            flow.build_connections()?;
        }
        FunctionProcess(ref mut function) => {
            config_function(
                function,
                &resolved_url.as_str(),
                parent_route,
                alias,
                parent_flow_id,
                lib_ref,
                initializations,
            )?;
        }
    }

    Ok(process)
}

/// load library metadata from the given url using the provider.
/// Currently it used the `package` table of Cargo.toml as a source but it could
/// easily use another file as along as it has the required fields to satisfy `MetaData` struct
pub fn load_metadata(url: &Url, provider: &dyn LibProvider) -> Result<MetaData> {
    trace!("Loading Metadata");
    let (resolved_url, _) = provider
        .resolve_url(url, "Cargo", &["toml"])
        .chain_err(|| format!("Could not resolve the url: '{}'", url))?;

    debug!("Source URL '{}' resolved to: '{}'", url, resolved_url);
    let contents = provider
        .get_contents(&resolved_url)
        .chain_err(|| format!("Could not get contents of resolved url: '{}'", resolved_url))?;

    let cargo: Cargo =
        toml::from_str(&String::from_utf8(contents).chain_err(|| "Could not read UTF8 contents")?)
            .chain_err(|| format!("Error deserializing Toml from: '{:?}'", resolved_url))?;

    Ok(cargo.package)
}

/*
    Configure a flow with additional information after it is deserialized from file
*/
fn config_flow(
    flow: &mut Flow,
    source_url: &Url,
    parent_route: &Route,
    alias_from_reference: &Name,
    id: usize,
    initializations: &Option<HashMap<String, InputInitializer>>,
) -> Result<()> {
    flow.id = id;
    flow.set_alias(alias_from_reference);
    flow.source_url = source_url.to_owned();
    IO::set_initial_values(flow.inputs_mut(), initializations);
    flow.set_routes_from_parent(parent_route);
    flow.validate()
}

/*
    Load sub-processes from the process_refs in a flow
*/
fn load_process_refs(
    flow: &mut Flow,
    flow_count: &mut usize,
    provider: &dyn LibProvider,
) -> Result<()> {
    if let Some(ref mut process_refs) = flow.process_refs {
        for process_ref in process_refs {
            let subprocess_url = flow
                .source_url
                .join(&process_ref.source)
                .map_err(|e| e.to_string())?;
            let process = load_process(
                &flow.route,
                &process_ref.alias(),
                flow.id,
                flow_count,
                &subprocess_url,
                provider,
                &process_ref.initializations,
            )?;
            process_ref.set_alias(process.name());

            // runtime needs references to library functions to be able to load the implementations at load time
            // library flow definitions are "compiled down" to just library function references at compile time.
            if let FunctionProcess(function) = &process {
                if let Some(lib_ref) = function.get_lib_reference() {
                    flow.lib_references.insert(
                        Url::parse(&format!("lib://{}/{}", lib_ref, function.name()))
                            .map_err(|_| "Could not create Url from library reference")?,
                    );
                }
            }

            flow.subprocesses
                .insert(process_ref.alias().to_owned(), process);
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn config_function(
    function: &mut Function,
    source_url: &str,
    parent_route: &Route,
    alias: &Name,
    flow_id: usize,
    lib_ref: Option<String>,
    initializations: &Option<HashMap<String, InputInitializer>>,
) -> Result<()> {
    function.set_flow_id(flow_id);
    function.set_alias(alias);
    function.set_source_url(source_url);
    function.set_lib_reference(lib_ref);
    function.set_routes_from_parent(parent_route);
    IO::set_initial_values(&mut function.inputs, initializations);
    function.validate()
}

#[cfg(test)]
mod test {
    use flowrstructs::manifest::{Cargo, MetaData};

    #[test]
    fn deserialize_library() {
        let contents = include_str!("../../../tests/test_libs/Cargo.toml");
        let cargo: Cargo = toml::from_str(contents)
            .expect("Could not parse Cargo.toml in deserialize_library test");
        let _: MetaData = cargo.package;
    }
}

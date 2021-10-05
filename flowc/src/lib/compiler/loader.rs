use std::collections::HashMap;
#[cfg(feature = "debugger")]
use std::collections::HashSet;

use log::{debug, info, trace};
use url::Url;

use flowcore::deserializers::deserializer::get_deserializer;
use flowcore::flow_manifest::{Cargo, MetaData};
use flowcore::input::InputInitializer;
use flowcore::lib_provider::Provider;

use crate::errors::*;
use crate::model::flow::Flow;
use crate::model::name::HasName;
use crate::model::name::Name;
use crate::model::process::Process;
use crate::model::process::Process::FlowProcess;
use crate::model::process::Process::FunctionProcess;
use crate::model::route::Route;

/// Many structs in the model implement the `Validate` method which is used to check the
/// description deserialized from file obeys some additional constraints that cannot be expressed
/// in the struct definition in `serde`
pub trait Validate {
    /// Validate that a deserialized model data structure is valid for use
    fn validate(&self) -> Result<()>;
}

/// Load a `Flow` definition from a `Url`, recursively loading all sub-processes referenced.
///
/// The return is a `Result` containing the `Process`, or a `String` describing the error
/// found while loading.
///
/// # Example
/// ```
/// use flowcore::lib_provider::Provider;
/// use flowcore::errors::Result;
/// use std::env;
/// use url::Url;
/// use std::collections::HashSet;
///
/// // Clients need to provide a Provider of content for the loader as flowlibc is independent of
/// // file systems and io.
/// struct DummyProvider;
///
/// // A Provider must implement the `Provider` trait, with the methods to `resolve` a URL and to
/// // `get` the contents for parsing.
/// impl Provider for DummyProvider {
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
/// // keep track of the source Urls loaded for this flow
/// let mut source_urls = HashSet::<(Url, Url)>::new();
///
/// // load the flow from `url = file:///example.toml` using the `dummy_provider`
/// flowclib::compiler::loader::load(&Url::parse("file:///example.toml").unwrap(), &dummy_provider, &mut source_urls).unwrap();
/// ```
pub fn load(
    url: &Url,
    provider: &dyn Provider,
    #[cfg(feature = "debugger")] source_urls: &mut HashSet<(Url, Url)>,
) -> Result<Process> {
    trace!("load()");
    load_process(
        &Route::default(),
        &Name::default(),
        0,
        &mut 0,
        url,
        provider,
        &HashMap::new(),
        #[cfg(feature = "debugger")]
        source_urls,
        0,
    )
}

#[allow(clippy::too_many_arguments)]
fn load_process(
    parent_route: &Route,
    alias: &Name,
    parent_flow_id: usize,
    flow_count: &mut usize,
    url: &Url,
    provider: &dyn Provider,
    initializations: &HashMap<String, InputInitializer>,
    #[cfg(feature = "debugger")] source_urls: &mut HashSet<(Url, Url)>,
    level: usize,
) -> Result<Process> {
    trace!("load_process()");

    let (resolved_url, lib_ref) = provider
        .resolve_url(url, "context", &["toml"])
        .chain_err(|| format!("Could not resolve the url: '{}'", url))?;
    if &resolved_url != url {
        debug!("Source URL '{}' resolved to: '{}'", url, resolved_url);
    }

    // Track the source file involved and what it resolved to
    #[cfg(feature = "debugger")]
    source_urls.insert((url.clone(), resolved_url.clone()));

    let contents = provider
        .get_contents(&resolved_url)
        .chain_err(|| format!("Could not get contents of resolved url: '{}'", resolved_url))?;

    if !alias.is_empty() {
        info!("Loading process with alias = '{}'", alias);
    }

    let content = String::from_utf8(contents).chain_err(|| "Could not read UTF8 contents")?;
    let deserializer = get_deserializer::<Process>(&resolved_url)?;
    debug!(
        "Loading process from url = '{}' with deserializer: '{}'",
        resolved_url,
        deserializer.name()
    );
    let mut process = deserializer
        .deserialize(&content, Some(url))
        .chain_err(|| format!("Could not deserialize process from content in '{}'", url))?;

    match process {
        FlowProcess(ref mut flow) => {
            flow.config(
                &resolved_url,
                parent_route,
                alias,
                *flow_count,
                initializations,
            )?;
            *flow_count += 1;
            debug!("Deserialized the Flow, now loading any sub-processes");
            load_process_refs(
                flow,
                flow_count,
                provider,
                #[cfg(feature = "debugger")]
                source_urls,
                level,
            )?;
            flow.build_connections(level)?;
        }
        FunctionProcess(ref mut function) => {
            function.config(
                resolved_url.as_str(),
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
/// Currently it uses the `package` table of Cargo.toml as a source but it could
/// easily use another file as along as it has the required fields to satisfy `MetaData` struct
pub fn load_metadata(url: &Url, provider: &dyn Provider) -> Result<MetaData> {
    trace!("Loading Metadata");
    let (resolved_url, _) = provider
        .resolve_url(url, "Cargo", &["toml"])
        .chain_err(|| format!("Could not resolve the url: '{}'", url))?;

    if &resolved_url != url {
        debug!("Source URL '{}' resolved to: '{}'", url, resolved_url);
    }

    let contents = provider
        .get_contents(&resolved_url)
        .chain_err(|| format!("Could not get contents of resolved url: '{}'", resolved_url))?;
    let content = String::from_utf8(contents).chain_err(|| "Could not read UTF8 contents")?;

    let deserializer = get_deserializer::<Cargo>(&resolved_url)?;

    let cargo: Cargo = deserializer.deserialize(&content, Some(&resolved_url))?;

    Ok(cargo.package)
}

/*
    Load sub-processes from the process_refs in a flow
*/
fn load_process_refs(
    flow: &mut Flow,
    flow_count: &mut usize,
    provider: &dyn Provider,
    #[cfg(feature = "debugger")] source_urls: &mut HashSet<(Url, Url)>,
    level: usize,
) -> Result<()> {
    for process_ref in &mut flow.process_refs {
        let subprocess_url = flow
            .source_url
            .join(&process_ref.source)
            .map_err(|e| e.to_string())?;
        let process = load_process(
            &flow.route,
            process_ref.alias(),
            flow.id,
            flow_count,
            &subprocess_url,
            provider,
            &process_ref.initializations,
            #[cfg(feature = "debugger")]
            source_urls,
            level + 1,
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

    Ok(())
}

#[cfg(test)]
mod test {
    use url::Url;

    use flowcore::deserializers::deserializer::get_deserializer;
    use flowcore::flow_manifest::{Cargo, MetaData};

    #[test]
    fn deserialize_library() {
        let cargo_toml = r###"[package]
name = "Flow Standard Library"
version = "0.11.0"
authors = ["Andrew Mackenzie <andrew@mackenzie-serres.net>"]
description = "The standard library for 'flow' programs compiled with the 'flowc' compiler"

exclude = "../..""###;
        let url = Url::parse("file:///fake.toml").expect("Could not parse URL");
        let deserializer = get_deserializer::<Cargo>(&url).expect("Could not get deserializer");
        let cargo: Cargo = deserializer
            .deserialize(cargo_toml, Some(&url))
            .expect("Could not deserialize");
        let _: MetaData = cargo.package;
    }
}

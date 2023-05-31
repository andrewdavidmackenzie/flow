#[cfg(feature = "debugger")]
use std::collections::BTreeMap;

use log::{debug, info, trace};
use url::Url;

use flowcore::deserializers::deserializer::get_deserializer;
use flowcore::model::flow_definition::FlowDefinition;
use flowcore::model::flow_manifest::Cargo;
use flowcore::model::input::InputInitializer;
use flowcore::model::metadata::MetaData;
use flowcore::model::name::HasName;
use flowcore::model::name::Name;
use flowcore::model::process::Process;
use flowcore::model::process::Process::FlowProcess;
use flowcore::model::process::Process::FunctionProcess;
use flowcore::model::route::Route;
use flowcore::provider::Provider;

use crate::errors::*;

/// `LibType` describes what format the Flow Library is written in
#[derive(PartialEq, Eq)]
pub enum LibType {
    /// `RustLib` indicates that the library is written in rust with a Cargo.toml to compile it natively
    RustLib,
}

/// Load a `Flow` definition from a `Url`, recursively loading all sub-processes referenced.
///
/// The return is a `Result` containing the `Process`, or a `String` describing the error
/// found while loading.
///
/// # Example
/// ```
/// use flowcore::provider::Provider;
/// use flowcore::errors::Result;
/// use std::env;
/// use url::Url;
/// use std::collections::BTreeMap;
///
/// // Clients need to provide a Provider of content for the loader as flowlibc is independent of
/// // file systems and io.
/// struct DummyProvider;
///
/// // A Provider must implement the `Provider` trait, with the methods to `resolve` a URL and to
/// // `get` the contents for parsing.
/// impl Provider for DummyProvider {
///     fn resolve_url(&self, url: &Url, default_filename: &str, _ext: &[&str]) -> Result<(Url, Option<Url>)> {
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
/// flowrclib::compiler::parser::parse(&Url::parse("file:///example.toml").unwrap(), &dummy_provider)
/// .unwrap();
/// ```
pub fn parse(
    url: &Url,
    provider: &dyn Provider,
) -> Result<Process> {
    parse_process(
        &Route::default(),
        &Name::default(),
        0,
        &mut 0,
        url,
        provider,
        &BTreeMap::new(),
        0,
    )
}

#[allow(clippy::too_many_arguments)]
fn parse_process(
    parent_route: &Route,
    alias: &Name,
    parent_flow_id: usize,
    flow_count: &mut usize,
    url: &Url,
    provider: &dyn Provider,
    initializations: &BTreeMap<String, InputInitializer>,
    level: usize,
) -> Result<Process> {
    let (resolved_url, reference) = provider
        .resolve_url(url, "root", &["toml"])
        .chain_err(|| format!("Could not resolve the url: '{url}'"))?;

    let contents = provider
        .get_contents(&resolved_url)
        .chain_err(|| format!("Could not get contents of resolved url: '{resolved_url}'"))?;

    if !alias.is_empty() {
        info!("Loading process with alias = '{alias}'");
    }

    let content = String::from_utf8(contents).chain_err(|| "Could not read UTF8 contents")?;
    let deserializer = get_deserializer::<Process>(&resolved_url)?;
    debug!(
        "Loading process from url = '{resolved_url}' with deserializer: '{}'", deserializer.name());
    let mut process = deserializer
        .deserialize(&content, Some(&resolved_url))
        .chain_err(|| format!("Could not parse a valid flow process from '{url}'"))?;

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
            debug!("Deserialized the Flow, now parsing sub-processes");
            parse_process_refs(
                flow,
                flow_count,
                provider,
                level,
            )?;
            flow.build_connections(level)?;
        }
        FunctionProcess(ref mut function) => {
            function.config(
                url,
                &resolved_url,
                parent_route,
                alias,
                parent_flow_id,
                reference,
                initializations,
            )?;
        }
    }

    Ok(process)
}

/// load library metadata from the given url using the provider.
/// Currently it uses the `package` table of Cargo.toml as a source but it could
/// easily use another file as along as it has the required fields to satisfy `MetaData` struct
pub fn parse_metadata(url: &Url, provider: &dyn Provider) -> Result<(MetaData, LibType)> {
    trace!("Loading Metadata");
    let (resolved_url, _) = provider
        .resolve_url(url, "Cargo", &["toml"])
        .chain_err(|| format!("Could not resolve the url: '{url}'"))?;

    if &resolved_url != url {
        debug!("Source URL '{url}' resolved to: '{resolved_url}'");
    }

    let contents = provider
        .get_contents(&resolved_url)
        .chain_err(|| format!("Could not get contents of resolved url: '{resolved_url}'"))?;
    let content = String::from_utf8(contents).chain_err(|| "Could not read UTF8 contents")?;

    let deserializer = get_deserializer::<Cargo>(&resolved_url)?;

    let cargo: Cargo = deserializer.deserialize(&content, Some(&resolved_url))?;

    Ok((cargo.package, LibType::RustLib))
}

/*
    Parse sub-processes from the process_refs in a flow
*/
fn parse_process_refs(
    flow: &mut FlowDefinition,
    flow_count: &mut usize,
    provider: &dyn Provider,
    level: usize,
) -> Result<()> {
    for process_ref in &mut flow.process_refs {
        let subprocess_url = flow
            .source_url
            .join(&process_ref.source)
            .map_err(|e| e.to_string())?;
        let process = parse_process(
            &flow.route,
            process_ref.alias(),
            flow.id,
            flow_count,
            &subprocess_url,
            provider,
            &process_ref.initializations,
            level + 1,
        )?;
        process_ref.set_alias(process.name());

        // runtime needs references to library functions to be able to load the implementations at load time
        // library flow definitions are "compiled down" to just library function references at compile time.
        if let FunctionProcess(function) = &process {
            if let Some(lib_ref) = function.get_lib_reference() {
                flow.lib_references.insert(lib_ref.clone());
            }

            if let Some(context_ref) = function.get_context_reference() {
                flow.context_references.insert(context_ref.clone());
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
    use flowcore::model::flow_manifest::Cargo;
    use flowcore::model::metadata::MetaData;

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

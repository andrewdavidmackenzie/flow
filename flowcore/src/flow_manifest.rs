use std::collections::HashSet;

use serde_derive::{Deserialize, Serialize};
use url::Url;

use crate::errors::*;
use crate::function::Function;
use crate::lib_provider::LibProvider;

/// The default name used for a flow Manifest file if none is specified
pub const DEFAULT_MANIFEST_FILENAME: &str = "manifest";

#[derive(Clone, Deserialize, Debug, Default, Serialize, PartialEq)]
/// `MetaData` about a `flow` that will be used in the flow's description and `Manifest`
pub struct MetaData {
    /// The human readable `name` of a `flow`
    #[serde(default)]
    pub name: String,
    /// Semantic versioning version number of the flow
    #[serde(default)]
    pub version: String,
    /// A description for humans
    #[serde(default)]
    pub description: String,
    /// The name of the people who authored the flow
    #[serde(default)]
    pub authors: Vec<String>,
}

#[derive(Clone, Deserialize, Serialize, PartialEq)]
/// `Cargo` meta-data that can be used as a source of meta-data
pub struct Cargo {
    /// We are only interested in the `package` part - as a source of meta-data
    pub package: MetaData,
}

#[derive(Deserialize, Serialize, Clone)]
/// A `flows` `Manifest` describes it and describes all the `Functions` it uses as well as
/// a list of references to libraries.
pub struct FlowManifest {
    /// The `MetaData` about this flow
    metadata: MetaData,
    /// A list of the `lib_references` used by this flow
    lib_references: HashSet<Url>,
    /// A list of descriptors of the `Functions` used in this flow
    functions: Vec<Function>,
    /// A list of the source files used to build this `flow`
    source_urls: HashSet<(Url, Url)>,
}

impl FlowManifest {
    /// Create a new manifest that can then be added to, and used in serialization
    pub fn new(metadata: MetaData) -> Self {
        FlowManifest {
            metadata,
            lib_references: HashSet::<Url>::new(),
            functions: Vec::<Function>::new(),
            source_urls: HashSet::<(Url, Url)>::new(),
        }
    }

    /// Add a run-time Function to the manifest for use in serialization
    pub fn add_function(&mut self, function: Function) {
        self.functions.push(function);
    }

    /// Get the list of functions in this manifest
    pub fn get_functions(&mut self) -> &mut Vec<Function> {
        &mut self.functions
    }

    /// Get the metadata structure for this manifest
    pub fn get_metadata(&self) -> &MetaData {
        &self.metadata
    }

    /// get the list of all library references in this manifest
    pub fn get_lib_references(&self) -> &HashSet<Url> {
        &self.lib_references
    }

    /// set the list of all library references in this manifest
    pub fn set_lib_references(&mut self, lib_references: &HashSet<Url>) {
        self.lib_references = lib_references.clone();
    }

    /// Add a new library reference (the name of a library) into the manifest
    pub fn add_lib_reference(&mut self, lib_reference: &Url) {
        self.lib_references.insert(lib_reference.clone());
    }

    /// set the list of all source urls used in the flow
    pub fn set_source_urls(&mut self, source_urls: HashSet<(Url, Url)>) {
        self.source_urls = source_urls;
    }

    /// Get the list of source files used in the flow
    pub fn get_source_urls(&self) -> &HashSet<(Url, Url)> {
        &self.source_urls
    }

    /// Load, or Deserialize, a manifest from a `source` Url using `provider`
    pub fn load(provider: &dyn LibProvider, source: &Url) -> Result<(FlowManifest, Url)> {
        let (resolved_url, _) = provider
            .resolve_url(source, DEFAULT_MANIFEST_FILENAME, &["json"])
            .chain_err(|| "Could not resolve url for manifest while attempting to load manifest")?;

        let content = provider
            .get_contents(&resolved_url)
            .chain_err(|| "Could not get contents while attempting to load manifest")?;

        // TODO for now json only
        let manifest = serde_json::from_str(
            &String::from_utf8(content).chain_err(|| "Could not convert from utf8 to String")?,
        )
        .chain_err(|| format!("Could not create a manifest from '{}'", source))?;

        Ok((manifest, resolved_url))
    }
}

#[cfg(test)]
mod test {
    use url::Url;

    use crate::errors::Result;
    use crate::function::Function;
    use crate::input::Input;
    use crate::lib_provider::LibProvider;

    use super::{FlowManifest, MetaData};

    fn test_meta_data() -> MetaData {
        MetaData {
            name: "test".into(),
            version: "0.0.0".into(),
            description: "a test".into(),
            authors: vec!["me".into()],
        }
    }

    pub struct TestProvider {
        test_content: &'static str,
    }

    impl LibProvider for TestProvider {
        fn resolve_url(
            &self,
            source: &Url,
            _default_filename: &str,
            _extensions: &[&str],
        ) -> Result<(Url, Option<String>)> {
            Ok((source.clone(), None))
        }

        fn get_contents(&self, _url: &Url) -> Result<Vec<u8>> {
            Ok(self.test_content.as_bytes().to_owned())
        }
    }

    #[test]
    fn create() {
        let _ = FlowManifest::new(test_meta_data());
    }

    fn test_function() -> Function {
        Function::new(
            #[cfg(feature = "debugger")]
            "test",
            #[cfg(feature = "debugger")]
            "/test",
            "file://fake/test",
            vec![Input::new(&None)],
            0,
            0,
            &[],
            false,
        )
    }

    #[test]
    fn add_function() {
        let function = test_function();

        let mut manifest = FlowManifest::new(test_meta_data());
        manifest.add_function(function);
        assert_eq!(manifest.functions.len(), 1);
    }

    #[test]
    fn load_manifest() {
        let test_content = "{
            \"metadata\": {
                \"name\": \"\",
                \"version\": \"0.1.0\",
                \"description\": \"\",
                \"authors\": []
                },
            \"manifest_dir\": \"fake dir\",
            \"lib_references\": [
                \"lib://flowstdlib\"
             ],
            \"functions\": [
                {
                    \"name\": \"print\",
                    \"route\": \"/print\",
                    \"id\": 0,
                    \"flow_id\": 0,
                    \"implementation_location\": \"lib://flowruntime/stdio/stdout/Stdout\",
                    \"inputs\": [ {} ]
                }
             ],
            \"source_urls\": []
            }";
        let provider = TestProvider { test_content };

        FlowManifest::load(
            &provider,
            &Url::parse("http://ibm.com").expect("Could not parse URL"),
        )
        .expect("Could not load manifest");
    }
}

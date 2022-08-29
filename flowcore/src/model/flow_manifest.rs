use std::collections::BTreeSet;

use serde_derive::{Deserialize, Serialize};
use url::Url;

use crate::deserializers::deserializer::get_deserializer;
use crate::errors::*;
use crate::meta_provider::Provider;
use crate::model::flow_definition::FlowDefinition;
use crate::model::metadata::MetaData;
use crate::model::runtime_function::RuntimeFunction;

/// The default name used for a flow Manifest file if none is specified
pub const DEFAULT_MANIFEST_FILENAME: &str = "manifest";

impl From<&FlowDefinition> for MetaData {
    fn from(flow: &FlowDefinition) -> Self {
        flow.metadata.clone()
    }
}

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
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
    lib_references: BTreeSet<Url>,
    /// A list of the `context_references` used by this flow
    context_references: BTreeSet<Url>,
    /// A list of descriptors of the `Functions` used in this flow
    functions: Vec<RuntimeFunction>,
    #[cfg(feature = "debugger")]
    /// A list of the source files used to build this `flow`
    source_urls: BTreeSet<(Url, Url)>,
}

impl FlowManifest {
    /// Create a new manifest that can then be added to, and used in serialization
    pub fn new(metadata: MetaData) -> Self {
        FlowManifest {
            metadata,
            lib_references: BTreeSet::<Url>::new(),
            context_references: BTreeSet::<Url>::new(),
            functions: Vec::<RuntimeFunction>::new(),
            #[cfg(feature = "debugger")]
            source_urls: BTreeSet::<(Url, Url)>::new(),
        }
    }

    /// Add a run-time Function to the manifest for use in serialization
    pub fn add_function(&mut self, function: RuntimeFunction) {
        self.functions.push(function);
    }

    /// Get the list of functions in this manifest
    pub fn get_functions(&mut self) -> &mut Vec<RuntimeFunction> {
        &mut self.functions
    }

    /// Get the metadata structure for this manifest
    pub fn get_metadata(&self) -> &MetaData {
        &self.metadata
    }

    /// get the list of all library references in this manifest
    pub fn get_lib_references(&self) -> &BTreeSet<Url> {
        &self.lib_references
    }

    /// get the list of all context references in this manifest
    pub fn get_context_references(&self) -> &BTreeSet<Url> {
        &self.context_references
    }

    /// set the list of all library references in this manifest
    pub fn set_lib_references(&mut self, lib_references: &BTreeSet<Url>) {
        self.lib_references = lib_references.clone();
    }

    /// set the list of all context references in this manifest
    pub fn set_context_references(&mut self, context_references: &BTreeSet<Url>) {
        self.context_references = context_references.clone();
    }

    /// Add a new library reference (the name of a library) into the manifest
    pub fn add_lib_reference(&mut self, lib_reference: &Url) {
        self.lib_references.insert(lib_reference.clone());
    }

    /// Add a new context reference (the name of a library) into the manifest
    pub fn add_context_reference(&mut self, context_reference: &Url) {
        self.context_references.insert(context_reference.clone());
    }

    /// set the list of all source urls used in the flow
    #[cfg(feature = "debugger")]
    pub fn set_source_urls(&mut self, source_urls: BTreeSet<(Url, Url)>) {
        self.source_urls = source_urls;
    }

    /// Get the list of source files used in the flow
    #[cfg(feature = "debugger")]
    pub fn get_source_urls(&self) -> &BTreeSet<(Url, Url)> {
        &self.source_urls
    }

    /// Load, or Deserialize, a manifest from a `source` Url using `provider`
    pub fn load(provider: &dyn Provider, source: &Url) -> Result<(FlowManifest, Url)> {
        let (resolved_url, _) = provider
            .resolve_url(source, DEFAULT_MANIFEST_FILENAME, &["json"])
            .chain_err(|| "Could not resolve url for manifest.json")?;

        let contents = provider
            .get_contents(&resolved_url)
            .chain_err(|| "Could not get contents while attempting to load manifest")?;

        let url = resolved_url.clone();
        let content =
            String::from_utf8(contents).chain_err(|| "Could not convert from utf8 to String")?;
        let deserializer = get_deserializer::<FlowManifest>(&resolved_url)?;
        let manifest = deserializer
            .deserialize(&content, Some(&resolved_url))
            .chain_err(|| format!("Could not create a FlowManifest from '{}'", source))?;

        // TODO normalize the relative ImplementationLocators into full file:// Urls here
        // using the manifest's resolved Url as the base...see executor.rs and avoid the need to
        // do there - then all locators can be treated equally. Maybe even add fields in the manifest
        // of type Url for them, and stop using the &str versions deserialized.
        // Custom deserializer for this?

        Ok((manifest, url))
    }
}

#[cfg(test)]
mod test {
    use url::Url;

    use crate::errors::Result;
    use crate::meta_provider::Provider;
    use crate::model::input::Input;
    use crate::model::runtime_function::RuntimeFunction;

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

    impl Provider for TestProvider {
        fn resolve_url(
            &self,
            source: &Url,
            _default_filename: &str,
            _extensions: &[&str],
        ) -> Result<(Url, Option<Url>)> {
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

    fn test_function() -> RuntimeFunction {
        RuntimeFunction::new(
            #[cfg(feature = "debugger")]
            "test",
            #[cfg(feature = "debugger")]
            "/test",
            "file://fake/test",
            vec![Input::new("", None, None)],
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
             ],
            \"context_references\": [
                \"context://\"
             ],
            \"functions\": [
                {
                    \"name\": \"print\",
                    \"route\": \"/print\",
                    \"function_id\": 0,
                    \"flow_id\": 0,
                    \"implementation_location\": \"context://stdio/stdout\",
                    \"inputs\": [ {} ]
                }
             ],
            \"source_urls\": []
            }";
        let provider = TestProvider { test_content };

        FlowManifest::load(
            &provider,
            &Url::parse("http://ibm.com/fake.json").expect("Could not parse URL"),
        )
        .expect("Could not load manifest");
    }
}

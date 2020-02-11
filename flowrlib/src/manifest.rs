use std::collections::HashSet;

use serde_derive::{Deserialize, Serialize};

use crate::errors::*;
use crate::function::Function;
use crate::provider::Provider;

/// The default name used for a flow Manifest file if none is specified
pub const DEFAULT_MANIFEST_FILENAME: &str = "manifest";

#[derive(Clone, Deserialize, Serialize, PartialEq)]
/// `MetaData` about a `flow` that will be used in the flow's `Manifest`
pub struct MetaData {
    /// The human readable `name` of a `flow`
    pub name: String,
    /// Semantic versioning version number of the flow
    pub version: String,
    /// A description for humans
    pub description: String,
    /// The name of the person who wrote the flow
    pub author_name: String,
    /// The email of the person who wrote the flow
    pub author_email: String,
}

#[derive(Deserialize, Serialize, Clone)]
/// A `flows` `Manifest` describes it and describes all the `Functions` it uses as well as
/// a list of references to libraries.
pub struct Manifest {
    /// The `MetaData` about this flow
    pub metadata: MetaData,
    /// A list of the `lib_references` used by this flow
    pub lib_references: HashSet<String>,
    /// A list of descriptors of the `Functions` used in this flow
    pub functions: Vec<Function>,
}

impl Manifest {
    /// Create a new manifest that can then be added to, and used in serialization
    pub fn new(metadata: MetaData) -> Self {
        Manifest {
            metadata,
            lib_references: HashSet::<String>::new(),
            functions: Vec::<Function>::new(),
        }
    }

    /// Add a run-time Function to the manifest for use in serialization
    pub fn add_function(&mut self, function: Function) {
        self.functions.push(function);
    }

    /// Load, or Deserialize, a manifest from a `source` Url using `provider`
    pub fn load(provider: &dyn Provider, source: &str) -> Result<Manifest> {
        let (resolved_url, _) = provider.resolve_url(source, DEFAULT_MANIFEST_FILENAME, &["json"])?;
        let content = provider.get_contents(&resolved_url)?;

        // TODO for now json only
        serde_json::from_str(
            &String::from_utf8(content).chain_err(|| "Could not convert from utf8 to String")?)
            .chain_err(|| format!("Could not create a manifest from '{}'", source))
    }
}

#[cfg(test)]
mod test {
    use crate::errors::*;
    use crate::function::Function;
    use crate::input::Input;
    use crate::provider::Provider;

    use super::{Manifest, MetaData};

    fn test_meta_data() -> MetaData {
        MetaData {
            name: "test".into(),
            version: "0.0.0".into(),
            description: "a test".into(),
            author_name: "me".into(),
            author_email: "me@a.com".into(),
        }
    }

    pub struct TestProvider {
        test_content: &'static str
    }

    impl Provider for TestProvider {
        fn resolve_url(&self, source: &str, _default_filename: &str, _extensions: &[&str]) -> Result<(String, Option<String>)> {
            Ok((source.to_string(), None))
        }

        fn get_contents(&self, _url: &str) -> Result<Vec<u8>> {
            Ok(self.test_content.as_bytes().to_owned())
        }
    }

    #[test]
    fn create() {
        let _ = Manifest::new(test_meta_data());
    }

    #[test]
    fn add_function() {
        let function = Function::new("test".to_string(),
                                         "/context/test".to_string(),
                                         "/test".to_string(),
                                         vec!(Input::new(1, &None, false)),
                                         0, 0,
                                         &vec!(), false);

        let mut manifest = Manifest::new(test_meta_data());
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
                \"author_name\": \"\",
                \"author_email\": \"\"
                },
            \"lib_references\": [
                \"lib://flowstdlib\"
             ],
            \"functions\": [
                {
                    \"name\": \"print\",
                    \"route\": \"/context/print\",
                    \"id\": 0,
                    \"flow_id\": 0,
                    \"implementation_location\": \"lib://flowruntime/stdio/stdout/Stdout\",
                    \"inputs\": [ {} ]
                }
             ]
            }";
        let provider = TestProvider {
            test_content
        };

        assert!(Manifest::load(&provider, "fake source").is_ok());
    }
}
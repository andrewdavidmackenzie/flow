use std::collections::HashSet;

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

#[derive(Deserialize, Serialize)]
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

    /// Add a runtime Function to the manifest for use in serialization
    pub fn add_function(&mut self, function: Function) {
        self.functions.push(function);
    }

    /// Load, or Deserialize, a manifest from a `source` Url using `provider`
    pub fn load(provider: &dyn Provider, source: &str) -> Result<Manifest> {
        let (resolved_url, _) = provider.resolve_url(source, DEFAULT_MANIFEST_FILENAME, &["json"])?;
        let content = provider.get_contents(&resolved_url)?;

        serde_json::from_str(
            &String::from_utf8(content).chain_err(|| "Could not convert from utf8 to String")?)
            .chain_err(|| format!("Could not create a manifest from '{}'", source))
    }
}
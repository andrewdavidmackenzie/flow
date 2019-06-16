use function::Function;
use provider::Provider;
use std::collections::HashSet;

pub const DEFAULT_MANIFEST_FILENAME: &str = "manifest.json";

#[derive(Clone, Deserialize, Serialize)]
pub struct MetaData {
    pub alias: String,
    pub version: String,
    pub author_name: String,
    pub author_email: String,
}

#[derive(Deserialize, Serialize)]
pub struct Manifest {
    pub metadata: MetaData,
    pub lib_references: HashSet<String>,
    pub functions: Vec<Function>,
}

impl Manifest {
    /*
        Create a new manifest that can then be added to, and used in serialization
    */
    pub fn new(metadata: MetaData) -> Self {
        Manifest {
            metadata,
            lib_references: HashSet::<String>::new(),
            functions: Vec::<Function>::new(),
        }
    }

    /*
        Add a runtime Function to the manifest for use in serialization
    */
    pub fn add_function(&mut self, function: Function) {
        self.functions.push(function);
    }

    /*
        Load, or Deserialize, a manifest from a `source` Url using `provider`
    */
    pub fn load(provider: &Provider, source: &str) -> Result<Manifest, String> {
        let (resolved_url, _) = provider.resolve(source, DEFAULT_MANIFEST_FILENAME)?;
        let content = provider.get(&resolved_url)?;

        serde_json::from_str(&String::from_utf8(content).unwrap())
            .map_err(|e| format!("Could not create a manifest from '{}'\nError = '{}'",
                                 source, e))
    }

    /*
        Deserialize a manifest from a content &str
    */
    pub fn from_str(content: &str) -> Result<Manifest, String> {
        serde_json::from_str(content)
            .map_err(|e| format!("Could not create a manifest from '{}'\nError = '{}'",
                                 content, e))
    }
}
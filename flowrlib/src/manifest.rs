use function::Function;
use provider::Provider;

pub const DEFAULT_MANIFEST_FILENAME: &str = "manifest.json";

#[derive(Clone, Deserialize, Serialize)]
pub struct MetaData {
    pub alias: String,
    pub version: String,
    pub author_name: String,
    pub author_email: String
}

#[derive(Deserialize, Serialize)]
pub struct Manifest {
    pub metadata: MetaData,
    pub functions: Vec<Function>
}

impl Manifest {
    /*
        Create a new manifest that can then be added to, and used in serialization
    */
    pub fn new(metadata: MetaData) -> Self {
        Manifest {
            metadata,
            functions: Vec::<Function>::new()
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
        let (resolved_url, _) = provider.resolve(source,DEFAULT_MANIFEST_FILENAME)?;
        let content = provider.get(&resolved_url)?;

        serde_json::from_str(&String::from_utf8(content).unwrap())
            .map_err(|e| format!("Could not read manifest from '{}'\nError = '{}'",
                                 source, e))
    }
}
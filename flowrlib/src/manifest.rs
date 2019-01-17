use super::process::Process;
use super::loader::Provider;

#[derive(Deserialize, Serialize)]
pub struct Manifest<'a> {
    pub processes: Vec<Process<'a>>
}

impl<'a> Manifest<'a> {
    pub fn new() -> Self {
        let processes=  Vec::<Process<'a>>::new();

        Manifest {
            processes
        }
    }

    pub fn load(provider: &Provider, path: &str) -> Result<Manifest<'a>, String> {
        let content = provider.get_content(path)?;

        serde_json::from_str(&content)
            .map_err(|e| format!("Could not deserialize manifest'{}'\nError = '{}'",
                                 path, e))
    }
}
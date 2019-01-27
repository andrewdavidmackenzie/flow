use process::Process;
use provider::Provider;

#[derive(Deserialize, Serialize)]
pub struct Manifest {
    pub processes: Vec<Process>
}

impl Manifest {
    pub fn new() -> Self {
        let processes=  Vec::<Process>::new();

        Manifest {
            processes
        }
    }

    pub fn load(provider: &Provider, source: &str) -> Result<Manifest, String> {
        let (resolved_url, _) = provider.resolve(source)?;
        let content = provider.get(&resolved_url)?;

        serde_json::from_str(&String::from_utf8(content).unwrap())
            .map_err(|e| format!("Could not read manifest from '{}'\nError = '{}'",
                                 source, e))
    }
}
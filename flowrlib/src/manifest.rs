use process::Process;
use provider::Provider;
use url::Url;

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

    pub fn load(provider: &Provider, url: &Url) -> Result<Manifest, String> {
        let (resolved_url, _) = provider.resolve(url)?;
        let content = provider.get(&resolved_url)?;

        serde_json::from_str(&content)
            .map_err(|e| format!("Could not read manifest from '{}'\nError = '{}'",
                                 url, e))
    }
}
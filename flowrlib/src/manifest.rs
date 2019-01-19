use process::Process;
use provider::Provider;
use url::Url;

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

    pub fn load(provider: &Provider, url: &Url) -> Result<Manifest<'a>, String> {
        let content = provider.get(&url)?;

        serde_json::from_str(&content)
            .map_err(|e| format!("Could not read manifest from '{}'\nError = '{}'",
                                 url, e))
    }
}
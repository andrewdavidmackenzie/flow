use url::Url;
use content::file_provider::FileProvider;

pub trait Provider {
    fn find(&self, url: &Url) -> Result<Url, String>;
    fn get(&self, url: &Url) -> Result<String, String>;
}

const FILE_PROVIDER: &Provider = &FileProvider as &Provider;

/*
    Accept a Url that:
     -  maybe url formatted with http/https, file, or lib
     That could point to:
     -  a specific file, that may or may not exist, try to open it
     -  a specific directory, that may or may not exist

     If no file is specified, then look for default file in a directory specified
*/
pub fn find(url: &Url) -> Result<Url, String> {
    let provider = get_provider(url)?;
    provider.find(url)
}

// Helper method to read the content of a file found at 'file_path' into a String result.
// 'file_path' could be absolute or relative, so we canonicalize it first...
pub fn get(url: &Url) -> Result<String, String> {
    let provider = get_provider(url)?;
    provider.get(url)
}

fn get_provider(url: &Url) -> Result<&'static Provider, String> {
    match url.scheme() {
        "file" => Ok(FILE_PROVIDER),
        _ => Err(format!("Cannot determine which provider to use for url: '{}'", url))
    }
}
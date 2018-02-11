use url::Url;
use content::file_provider::FileProvider;
use content::lib_provider::LibProvider;
use content::http_provider::HttpProvider;

pub trait Provider {
    /// 'resolve' takes a Url and uses it to determine a url where actual content can be read from
    /// using some provider specific logic. This may involve looking for default files in a
    /// directory (a file provider) or a server path (an http provider), or it may involve
    /// translating a virtual Url into a real on where content can be found (lib provider).
    fn resolve(&self, url: &Url) -> Result<(Url, Option<String>, Option<String>), String>;

    /// 'get' fetches content from a url. It resolves the url internally before attempting to
    /// fetch actual content
    fn get(&self, url: &Url) -> Result<String, String>;
}

const FILE_PROVIDER: &Provider = &FileProvider as &Provider;
const LIB_PROVIDER: &Provider = &LibProvider as &Provider;
const HTTP_PROVIDER: &Provider = &HttpProvider as &Provider;

/*
    Accept a Url that:
     -  maybe url formatted with http/https, file, or lib
     That could point to:
     -  a specific file, that may or may not exist, try to open it
     -  a specific directory, that may or may not exist

     If no file is specified, then look for default file in a directory specified
*/
fn resolve(url: &Url) -> Result<(Url, Option<String>, Option<String>), String> {
    let provider = get_provider(url)?;
    provider.resolve(url)
}

// Helper method to read the content of a file found at 'file_path' into a String result,
// plus an optional library name, and an optional function path in the library
// 'file_path' could be absolute or relative, so we canonicalize it first...
pub fn get(url: &Url) -> Result<(String, Option<String>, Option<String>), String> {
    let (resolved_url, lib_name, lib_ref) = resolve(url)?;
    // The 'resolved' Url maybe served by a different provider
    let provider = get_provider(&resolved_url)?;
    let content = provider.get(&resolved_url)?;
    Ok((content, lib_name, lib_ref))
}

fn get_provider(url: &Url) -> Result<&'static Provider, String> {
    match url.scheme() {
        "file" => Ok(FILE_PROVIDER),
        "lib" => Ok(LIB_PROVIDER),
        "http"|"https" => Ok(HTTP_PROVIDER),
        _ => Err(format!("Cannot determine which provider to use for url: '{}'", url))
    }
}
use url::Url;
use content::file_provider::FileProvider;
use content::lib_provider::LibProvider;
use content::http_provider::HttpProvider;

pub trait Provider {
    /// 'resolve' takes a Url and uses it to determine a url where actual content can be read from
    /// using some provider specific logic. This may involve looking for default files in a
    /// directory (a file provider) or a server path (an http provider), or it may involve
    /// translating a virtual Url into a real on where content can be found (lib provider).
    /// It also returns an optional String which is a library reference in case that applies.
    fn resolve(&self, url: &Url) -> Result<(Url, Option<String>), String>;

    /// 'get' fetches content from a url. It resolves the url internally before attempting to
    /// fetch actual content
    fn get(&self, url: &Url) -> Result<String, String>;
}

const FILE_PROVIDER: &Provider = &FileProvider as &Provider;
const LIB_PROVIDER: &Provider = &LibProvider as &Provider;
const HTTP_PROVIDER: &Provider = &HttpProvider as &Provider;

// Determine which provider should be used based on the scheme of the Url of the content
fn get_provider(url: &Url) -> Result<&'static Provider, String> {
    match url.scheme() {
        "file" => Ok(FILE_PROVIDER),
        "lib" => Ok(LIB_PROVIDER),
        "http"|"https" => Ok(HTTP_PROVIDER),
        _ => Err(format!("Cannot determine which provider to use for url: '{}'", url))
    }
}

// Resolve a content Url to determine where the content should attempt to be loaded from.
fn resolve(url: &Url) -> Result<(Url, Option<String>), String> {
    let provider = get_provider(url)?;
    provider.resolve(url)
}

/// Takes a Url with a scheme of "http", "https", "file", or "lib"
/// Which could refer to:
///     -  a specific file or flow (that may or may not exist)
///         - in which case, confirm it exists and can be read then return it's contents
///     -  a directory (that may or may not exist)
///         - in which we should look for a default file (the name of the default file is
///           provider specific) and confirm it exists and can be opened, read and return contents
///     - a reference to a file in a library, transform the reference into a Url where the content
///        can be found in the file system, and read and return the contents
pub fn get(url: &Url) -> Result<(String, Option<String>), String> {
    let (resolved_url, lib_ref) = resolve(url)?;
    // The 'resolved' Url maybe served by a different provider
    let provider = get_provider(&resolved_url)?;
    let content = provider.get(&resolved_url)?;
    Ok((content, lib_ref))
}
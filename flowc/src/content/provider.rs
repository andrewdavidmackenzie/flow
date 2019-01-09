use url::Url;
use content::file_provider::FileProvider;
use content::lib_provider::LibProvider;
use content::http_provider::HttpProvider;
use flowclib::loader::provider::Provider;

const FILE_PROVIDER: &Provider = &FileProvider as &Provider;
const LIB_PROVIDER: &Provider = &LibProvider as &Provider;
const HTTP_PROVIDER: &Provider = &HttpProvider as &Provider;

pub struct MetaProvider {}

impl MetaProvider {
    // Determine which provider should be used based on the scheme of the Url of the content
    fn get_provider(url: &Url) -> Result<&'static Provider, String> {
        match url.scheme() {
            "file" => Ok(FILE_PROVIDER),
            "lib" => Ok(LIB_PROVIDER),
            "http" | "https" => Ok(HTTP_PROVIDER),
            _ => Err(format!("Cannot determine which provider to use for url: '{}'", url))
        }
    }
}

impl Provider for MetaProvider {
    /// Takes a Url with a scheme of "http", "https", "file", or "lib" and determine where the content
    /// should be loaded from.
    ///
    /// Url could refer to:
    ///     -  a specific file or flow (that may or may not exist)
    ///     -  a directory - if exists then look for a provider specific default file
    ///     -  a file in a library, transform the reference into a Url where the content can be found
    fn resolve(&self, url: &Url) -> Result<(Url, Option<String>), String> {
        let provider = Self::get_provider(url)?;
        provider.resolve(url)
    }

    /// Takes a Url with a scheme of "http", "https" or "file". Read and return the contents of the
    /// resource at that Url.
    fn get(&self, url: &Url) -> Result<String, String> {
        let provider = Self::get_provider(&url)?;
        let content = provider.get(&url)?;
        Ok(content)
    }
}

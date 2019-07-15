//! MetaProvider is an overall provider that determines which internal content provider to use
//! based on the URL scheme provided (http, file or lib).
use flowrlib::errors::*;
use flowrlib::provider::Provider;
use url::Url;

use crate::content::file_provider::FileProvider;
use crate::content::http_provider::HttpProvider;
use crate::content::lib_provider::LibProvider;

const FILE_PROVIDER: &Provider = &FileProvider as &Provider;
const LIB_PROVIDER: &Provider = &LibProvider as &Provider;
const HTTP_PROVIDER: &Provider = &HttpProvider as &Provider;

pub struct MetaProvider {}

///
/// // Instantiate MetaProvider and then use the Provider trait methods on it to resolve and fetch
/// // content depending on the URL scheme.
/// let meta_provider = MetaProvider{};
/// let url = "file://directory";
/// let (resolved_url, lib_ref) = meta_provider.resolve(url, "default.toml")?;
/// let contents = meta_provider.get(&resolved_url)?;
///
impl MetaProvider {
    // Determine which specific provider should be used based on the scheme of the Url of the content
    fn get_provider(url_str: &str) -> Result<&'static Provider> {
        let url = Url::parse(url_str)
            .chain_err(|| format!("Could not convert '{}' to valid Url", url_str))?;
        match url.scheme() {
            "" => Ok(FILE_PROVIDER),
            "file" => Ok(FILE_PROVIDER),
            "lib" => Ok(LIB_PROVIDER),
            "http" | "https" => Ok(HTTP_PROVIDER),
            _ => bail!("Cannot determine which provider to use for url: '{}'", url)
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
    fn resolve(&self, url: &str, default_filename: &str) -> Result<(String, Option<String>)> {
        let provider = Self::get_provider(url)?;
        provider.resolve(url, default_filename)
    }

    /// Takes a Url with a scheme of "http", "https" or "file". Read and return the contents of the
    /// resource at that Url.
    fn get(&self, url: &str) -> Result<Vec<u8>> {
        let provider = Self::get_provider(&url)?;
        let content = provider.get(&url)?;
        Ok(content)
    }
}

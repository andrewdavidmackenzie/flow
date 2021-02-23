//! The Provider trait must be implemented by Providers of content to the compiler or runtime
use simpath::Simpath;
use url::Url;

use crate::content::file_provider::FileProvider;
use crate::content::http_provider::HttpProvider;
use crate::content::lib_provider::LibProvider;
use crate::errors::*;

/// A content provider is responsible with interfacing with the environment and doing IO
/// or what is required to supply content related with flows - isolating other libraries
/// from the File SSystem or IO. It must implement the `Provider` trait
pub trait Provider {
    /// Take a URL and uses it to determine a url where actual content can be read from
    /// using some provider specific logic. This may involve looking for default files in a
    /// directory (a file provider) or a server path (an http provider), or it may involve
    /// translating a virtual URL into a real on where content can be found (lib provider).
    /// It also returns an optional String which is a library reference in case that applies.
    fn resolve_url(&self, url: &str, default_file: &str, extensions: &[&str]) -> Result<(String, Option<String>)>;

    /// Fetches content from a URL. It resolves the URL internally before attempting to
    /// fetch actual content
    fn get_contents(&self, url: &str) -> Result<Vec<u8>>;
}

const FILE_PROVIDER: &dyn Provider = &FileProvider as &dyn Provider;
const HTTP_PROVIDER: &dyn Provider = &HttpProvider as &dyn Provider;

/// The `MetaProvider` implements the `Provider` trait and based on the url and it's
/// resolution to a real location for content invokes one of the child providers it has
/// to fetch the content (e.g. File or Http).
pub struct MetaProvider {
    lib_provider: LibProvider
}

/// Instantiate MetaProvider and then use the Provider trait methods on it to resolve and fetch
/// content depending on the URL scheme.
/// ```
/// use provider::content::provider::{Provider, MetaProvider};
/// use simpath::Simpath;
/// let lib_search_path = Simpath::new("FLOW_LIB_PATH");
/// let meta_provider = &MetaProvider::new(lib_search_path) as &dyn Provider;
/// let url = "file://directory";
/// match meta_provider.resolve_url(url, "default", &["toml"]) {
///     Ok((resolved_url, lib_ref)) => {
///         match meta_provider.get_contents(&resolved_url) {
///             Ok(contents) => println!("Content: {:?}", contents),
///             Err(e) => println!("Got error '{}'", e)
///         }
///     }
///     Err(e) => {
///         println!("Found error '{}'", e);
///     }
/// };
/// ```
impl MetaProvider {
    pub fn new(lib_search_path: Simpath) -> Self {
        MetaProvider {
            lib_provider: LibProvider::new(lib_search_path)
        }
    }

    // Determine which specific provider should be used based on the scheme of the Url of the content
    fn get_provider(&self, url_str: &str) -> Result<&dyn Provider> {
        let url = Url::parse(url_str)
            .chain_err(|| format!("Could not convert '{}' to valid Url", url_str))?;
        match url.scheme() {
            "" => Ok(FILE_PROVIDER),
            "file" => Ok(FILE_PROVIDER),
            "lib" => Ok(&self.lib_provider as &dyn Provider),
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
    fn resolve_url(&self, url: &str, default_filename: &str, extensions: &[&str]) -> Result<(String, Option<String>)> {
        let provider = self.get_provider(url)?;
        provider.resolve_url(url, default_filename, extensions)
    }

    /// Takes a Url with a scheme of "http", "https" or "file". Read and return the contents of the
    /// resource at that Url.
    fn get_contents(&self, url: &str) -> Result<Vec<u8>> {
        let provider = self.get_provider(&url)?;
        let content = provider.get_contents(&url)?;
        Ok(content)
    }
}

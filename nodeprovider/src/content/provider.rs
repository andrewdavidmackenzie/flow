//! MetaProvider is an overall provider that determines which internal content provider to use
//! based on the URL scheme provided (http, file or lib).
use flowrlib::errors::*;
use flowrlib::provider::Provider;

use crate::content::file_provider::FileProvider;
use crate::content::http_provider::HttpProvider;
use crate::content::lib_provider::LibProvider;

const HTTP_PROVIDER: &dyn Provider = &HttpProvider as &dyn Provider;

/// The `MetaProvider` implements the `Provider` trait and based on the url and it's
/// resolution to a real location for content invokes one of the child providers it has
/// to fetch the content (e.g. File or Http).
pub struct MetaProvider {
    file_provider: FileProvider,
    lib_provider: LibProvider
}

/// Instantiate MetaProvider and then use the Provider trait methods on it to resolve and fetch
/// content depending on the URL scheme.
/// ```
/// use nodeprovider::content::provider::MetaProvider;
/// use flowrlib::provider::Provider;
///
/// let meta_provider = &MetaProvider::new("Fake Contnet".into(), "./".into()) as &dyn Provider;
/// let url = "file://directory";
/// match meta_provider.resolve_url(url, "default", &["toml"]) {
///     Ok((resolved_url, lib_ref)) => {
///         match meta_provider.get_contents(&resolved_url) {
///             Ok(contents) => println!("Content: {:?}", contents),
///             Err(e) => println!("Got error '{}'", e)
///         }
///     },
///     Err(e) => {
///         println!("Found error '{}'", e);
///     }
/// };
/// ```
impl MetaProvider {
    /// Create a new nodeprovider - which currently takes a String as faked content plus a `Path`
    /// to be searched (as a String)
    pub fn new(content: String, flow_lib_path: String) -> Self {
        MetaProvider{
            file_provider: FileProvider::new(content),
            lib_provider: LibProvider::new(flow_lib_path)
        }
    }

    // Determine which specific provider should be used based on the scheme of the Url of the content
    fn get_provider(&self, url_str: &str) -> Result<&dyn Provider> {
        let parts: Vec<&str> = url_str.split(":").collect();
        let scheme = parts[0];
        info!("Looking for correct content provider for scheme: '{}'", scheme);
        match scheme {
            "" => Ok(&self.file_provider),
            "file" => Ok(&self.file_provider),
            "lib" => Ok(&self.lib_provider),
            "http" | "https" => Ok(HTTP_PROVIDER),
            _ => bail!("Cannot determine which provider to use for url: '{}'", url_str)
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

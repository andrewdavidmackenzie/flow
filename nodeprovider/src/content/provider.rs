//! MetaProvider is an overall provider that determines which internal content provider to use
//! based on the URL scheme provided (http, file or lib).
use content::file_provider::FileProvider;
use content::http_provider::HttpProvider;
use content::lib_provider::LibProvider;
use flowrlib::provider::Provider;

const HTTP_PROVIDER: &Provider = &HttpProvider as &Provider;

pub struct MetaProvider {
    file_provider: FileProvider,
    lib_provider: LibProvider
}

///
/// // Instantiate MetaProvider and then use the Provider trait methods on it to resolve and fetch
/// // content depending on the URL scheme.
/// let meta_provider = MetaProvider{};
/// let url = "file://directory";
/// let (resolved_url, lib_ref) = meta_provider.resolve(url, "default.toml")?;
/// let contents = meta_provider.get(&resolved_url)?;
///
impl MetaProvider {
    pub fn new(content: String, flow_lib_path: String) -> Self {
        MetaProvider{
            file_provider: FileProvider::new(content),
            lib_provider: LibProvider::new(flow_lib_path)
        }
    }

    // Determine which specific provider should be used based on the scheme of the Url of the content
    fn get_provider(&self, url_str: &str) -> Result<&Provider, String> {
        let parts: Vec<&str> = url_str.split(":").collect();
        let scheme = parts[0];
        info!("Looking for correct content provider for scheme: '{}'", scheme);
        match scheme {
            "" => Ok(&self.file_provider),
            "file" => Ok(&self.file_provider),
            "lib" => Ok(&self.lib_provider),
            "http" | "https" => Ok(HTTP_PROVIDER),
            _ => Err(format!("Cannot determine which provider to use for url: '{}'", url_str))
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
    fn resolve(&self, url: &str, default_filename: &str) -> Result<(String, Option<String>), String> {
        let provider = self.get_provider(url)?;
        provider.resolve(url, default_filename)
    }

    /// Takes a Url with a scheme of "http", "https" or "file". Read and return the contents of the
    /// resource at that Url.
    fn get(&self, url: &str) -> Result<Vec<u8>, String> {
        let provider = self.get_provider(&url)?;
        let content = provider.get(&url)?;
        Ok(content)
    }
}

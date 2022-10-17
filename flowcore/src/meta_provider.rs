#[cfg(any(feature = "context"))]
use std::path::PathBuf;

#[cfg(feature = "file_provider")]
use simpath::{FoundType, Simpath};
use url::Url;

#[cfg(feature = "file_provider")]
use crate::content::file_provider::FileProvider;
#[cfg(feature = "http_provider")]
use crate::content::http_provider::HttpProvider;
#[cfg(feature = "p2p_provider")]
use crate::content::p2p_provider::P2pProvider;
use crate::errors::*;
use crate::provider::Provider;

#[cfg(feature = "file_provider")]
const FILE_PROVIDER: &dyn Provider = &FileProvider as &dyn Provider;
#[cfg(feature = "http_provider")]
const HTTP_PROVIDER: &dyn Provider = &HttpProvider as &dyn Provider;
#[cfg(feature = "p2p_provider")]
const P2P_PROVIDER: &dyn Provider = &P2pProvider as &dyn Provider;

/// The `MetaProvider` implements the `Provider` trait and based on the url and it's
/// resolution to a real location for content invokes one of the child providers it has
/// to fetch the content (e.g. File or Http).
pub struct MetaProvider {
    #[cfg(feature = "file_provider")]
    lib_search_path: Simpath,
    #[cfg(feature = "context")]
    context_root: PathBuf,
}

/// Instantiate MetaProvider and then use the Provider trait methods on it to resolve and fetch
/// content depending on the URL scheme.
/// ```
/// #[cfg(feature = "context")]
/// use std::path::PathBuf;
/// use simpath::Simpath;
/// use url::Url;
/// use flowcore::provider::Provider;
/// use flowcore::meta_provider::MetaProvider;
/// #[cfg(feature = "file_provider")]
/// let lib_search_path = Simpath::new_with_separator("FLOW_LIB_PATH", ',');
/// let meta_provider = &mut MetaProvider::new(
///                                             #[cfg(feature = "file_provider")]lib_search_path,
///                                             #[cfg(feature = "context")] PathBuf::from("/")
///                                             ) as &dyn Provider;
/// let url = Url::parse("file://directory").unwrap();
/// match meta_provider.resolve_url(&url, "default", &["toml"]) {
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
    /// Create a new `MetaProvider` initializing it with:
    /// - a search path where to look for libraries
    /// - the root of the context functions provided by the runtime (requires "context" feature
    pub fn new(
                #[cfg(feature = "file_provider")] lib_search_path: Simpath,
                #[cfg(feature = "context")] context_root: PathBuf
                ) -> Self {
        MetaProvider {
            #[cfg(feature = "file_provider")] lib_search_path,
            #[cfg(feature = "context")] context_root
        }
    }

    // Determine which specific provider should be used based on the scheme of the Url of the content
    fn get_provider(&self, scheme: &str) -> Result<&dyn Provider> {
        match scheme {
            #[cfg(all(not(target_arch = "wasm32"), feature = "file_provider"))]
            "file" => Ok(FILE_PROVIDER),
            #[cfg(all(not(target_arch = "wasm32"), feature = "http_provider"))]
            "http" | "https" => Ok(HTTP_PROVIDER),
            #[cfg(all(not(target_arch = "wasm32"), feature = "p2p_provider"))]
            "p2p" => Ok(P2P_PROVIDER),
            _ => bail!("Cannot determine which provider to use for url with scheme: 'scheme'"),
        }
    }

    /// Urls for context functions will be of the form:
    ///        "context://directory/subdirectory"
    ///
    /// The context function in question is found in the file system under the context_root
    /// directory, which is supplied in the constructor by the host runtime using this lib
    ///
    ///   Then return:
    ///    - a string representation of the Url (file: or http: or https:) where the file can be found
    ///    - a string that is a reference to that module in the library, such as:
    ///        "context/stdio/stdout/stdout"
    #[cfg(feature = "context")]
    fn resolve_context_url(&self, url: &Url) -> Result<(Url, Option<Url>)> {
        let dir = url.host_str()
            .chain_err(|| format!("context 'dir' could not be extracted from the url '{}'", url))?;
        let sub_dir = url.path().trim_start_matches('/');
        let context_function_path = self.context_root.join(dir).join(sub_dir);
        Ok((
            Url::from_file_path(context_function_path)
                .map_err(|_| "Could not convert context function's path to a Url")?,
            Some(Url::parse(&format!("context://{}/{}", dir, sub_dir))?),
        ))
    }

    /// Urls for library flows and functions and values will be of the form:
    ///        "lib://flowstdlib/stdio/stdout.toml"
    ///
    ///    Where 'flowstdlib' is the library name and 'stdio/stdout.toml' the path of the definition
    ///    file within the library.
    ///
    ///   Find library in question is found in the file system or via Http using the provider's
    ///   search path (setup on provider creation).
    ///
    ///   Then return:
    ///    - a string representation of the Url (file: or http: or https:) where the file can be found
    ///    - a string that is a reference to that module in the library, such as:
    ///        "flowstdlib/math/add"
    #[cfg(feature = "file_provider")]
    fn resolve_lib_url(&self, url: &Url) -> Result<(Url, Option<Url>)> {
        let lib_name = url.host_str()
            .chain_err(|| format!("'lib_name' could not be extracted from the url '{}'", url))?;
        let path_under_lib = url.path().trim_start_matches('/');
        let lib_reference = Some(Url::parse(&format!("lib://{}/{}", lib_name, path_under_lib))?);

        match self.lib_search_path.find(lib_name) {
            Ok(FoundType::File(lib_root_path)) => {
                let lib_path = lib_root_path.join(path_under_lib);
                Ok((
                    Url::from_directory_path(lib_path)
                        .map_err(|_| "Could not convert file: lib_path to Url")?,
                    lib_reference,
                ))
            }
            Ok(FoundType::Resource(mut lib_root_url)) => {
                lib_root_url.set_path(&format!("{}/{}", lib_root_url.path(), path_under_lib));
                Ok((lib_root_url, lib_reference))
            }
            _ => bail!(
                "Could not resolve library Url '{}' using {}",
                url,
                self.lib_search_path
            ),
        }
    }
}

impl Provider for MetaProvider {
    /// Takes a Url with a scheme of "http", "https", "file", "lib" or "context" and determine
    /// where the content should be loaded from.
    ///
    /// Url could refer to:
    ///     -  a specific file or flow (that may or may not exist)
    ///     -  a directory - if exists then look for a provider specific default file
    ///     -  a file in a library, transform the reference into a Url where the content can be found
    fn resolve_url(
        &self,
        url: &Url,
        default_name: &str,
        extensions: &[&str],
    ) -> Result<(Url, Option<Url>)> {
        // resolve a lib reference into either a file: or http: or https: reference
        let (content_url, reference) = match url.scheme() {
            #[cfg(feature = "file_provider")]
            "lib" => self.resolve_lib_url(url)?,
            #[cfg(feature = "context")]
            "context" => self.resolve_context_url(url)?,
            _ => (url.clone(), None),
        };

        let provider = self.get_provider(content_url.scheme())?;
        let (resolved_url, _) = provider.resolve_url(&content_url, default_name, extensions)?;

        Ok((resolved_url, reference))
    }

    /// Takes a Url with a scheme of "http", "https" or "file". Read and return the contents of the
    /// resource at that Url.
    fn get_contents(&self, url: &Url) -> Result<Vec<u8>> {
        let scheme = url.scheme().to_string();
        let provider = self.get_provider(&scheme)?;
        let content = provider.get_contents(url)?;
        Ok(content)
    }
}

#[cfg(test)]
mod test {
    #[cfg(feature = "file_provider")]
    use std::path::Path;
    #[cfg(feature = "context")]
    use std::path::PathBuf;

    #[cfg(feature = "file_provider")]
    use simpath::Simpath;
    #[cfg(any(feature = "file_provider", feature = "http_provider"))]
    use url::Url;

    use super::MetaProvider;
    #[cfg(any(feature = "file_provider", feature = "http_provider"))]
    use super::Provider;

    #[test]
    fn get_invalid_provider() {
        #[cfg(feature = "file_provider")]
        let search_path = Simpath::new("TEST");
        let meta = MetaProvider::new(
                                    #[cfg(feature = "file_provider")] search_path,
                                        #[cfg(feature = "context")]
                                         PathBuf::from("/")
        );

        assert!(meta.get_provider("fake://bla").is_err());
    }

    #[cfg(feature = "http_provider")]
    #[test]
    fn get_http_provider() {
        let search_path = Simpath::new("TEST");
        let meta = MetaProvider::new(search_path,
                                     #[cfg(feature = "context")]
                                     PathBuf::from("/")
        );

        assert!(meta.get_provider("http").is_ok());
    }

    #[cfg(feature = "http_provider")]
    #[test]
    fn get_https_provider() {
        let search_path = Simpath::new("TEST");
        let meta = MetaProvider::new(search_path,
                                     #[cfg(feature = "context")]
                                     PathBuf::from("/")
        );

        assert!(meta.get_provider("https").is_ok());
    }

    #[cfg(feature = "file_provider")]
    #[test]
    fn get_file_provider() {
        let search_path = Simpath::new("TEST");
        let meta = MetaProvider::new(search_path,
                                     #[cfg(feature = "context")]
                                     PathBuf::from("/")
        );

        assert!(meta.get_provider("file").is_ok());
    }

    #[cfg(feature = "file_provider")]
    fn get_lib_search_path() -> Simpath {
        let mut lib_search_path = Simpath::new("lib_search_path");
        let tests_str = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests");
        lib_search_path.add_directory(
            tests_str
                .to_str()
                .expect("Could not get tests/ path as string"),
        );
        println!("{}", lib_search_path);
        lib_search_path
    }

    #[cfg(feature = "file_provider")]
    #[test]
    fn resolve_path() {
        let root_str = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("Could not get project root dir");
        let expected_url = Url::parse(&format!(
            "file://{}/flowcore/tests/test-flows/control/compare_switch/compare_switch.toml",
            root_str.display()
        ))
        .expect("Could not create expected url");
        let provider = &MetaProvider::new(get_lib_search_path(),
                                          #[cfg(feature = "context")]
                                          PathBuf::from("/")
        ) as &dyn Provider;
        let lib_url = Url::parse("lib://test-flows/control/compare_switch").expect("Couldn't form Url");
        match provider.resolve_url(&lib_url, "", &["toml"]) {
            Ok((url, lib_ref)) => {
                assert_eq!(url, expected_url);
                assert_eq!(lib_ref, Some(Url::parse("lib://test-flows/control/compare_switch")
                    .expect("Could not parse Url")));
            }
            Err(_) => panic!("Error trying to resolve url"),
        }
    }

    #[cfg(all(feature = "http_provider", feature = "online_tests"))]
    #[test]
    fn resolve_web_path() {
        let mut search_path = Simpath::new("web_path");
        // `flowstdlib` can be found under the root of the project at `tree/master/flowstdlib` on github
        search_path.add_url(
            &Url::parse(
                "https://raw.githubusercontent.com/andrewdavidmackenzie/flow/master/flowstdlib",
            )
            .expect("Could not parse the url for Simpath"),
        );

        let expected_url = Url::parse("https://raw.githubusercontent.com/andrewdavidmackenzie/flow/master/flowstdlib/control/tap/tap.toml")
            .expect("Couldn't parse expected Url");

        let provider = &MetaProvider::new(search_path,
                                          #[cfg(feature = "context")]
                                          PathBuf::from("/")
        );

        let lib_url = Url::parse("lib://flowstdlib/control/tap").expect("Couldn't create Url");
        let (resolved_url, _) = provider
            .resolve_url(&lib_url, "", &["toml"])
            .expect("Couldn't resolve library on the web");
        assert_eq!(resolved_url, expected_url);
    }
}

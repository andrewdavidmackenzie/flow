use std::path::PathBuf;

use log::debug;
use simpath::{FoundType, Simpath};
use url::Url;

use crate::content::file_provider::FileProvider;
use crate::errors::*;

use super::provider::Provider;

pub struct LibProvider {
    lib_search_path: Simpath
}

impl LibProvider {
    pub fn new(lib_search_path: Simpath) -> Self {
        LibProvider {
            lib_search_path
        }
    }

    /*
        If the library is located as a directory, then construct a PathBuf to refer to the definition file:
        - either "stdio/stdout.toml" or
        - "stdio/stdout/stdout.toml"

        within the library (using knowledge of library file structure).

        If the file exists, then create a "file:" Url that points to the file, for the file provider
        to use later to read the content.
     */
    fn resolve_file_path(url: &Url, lib_root_path: &mut PathBuf, lib_name: &str,
                         default_filename: &str, extensions: &[&str]) -> Result<(String, Option<String>)> {
        // Once we've found the path where the library resides, append the rest of the
        // url path to it, to form a path to the directory where the process being loaded resides
        if !url.path().is_empty() {
            lib_root_path.push(&url.path()[1..]);
        }

        // // Drop the file extension off the lib definition file path to get a lib reference
        let module = url.join("./")
            .chain_err(|| "Could not perform join")?
            .join(lib_root_path.file_stem()
                .chain_err(|| "Could not get file stem")?
                .to_str()
                .chain_err(|| "Could not convert file stem to string")?)
            .chain_err(|| "Could not create module Url")?;
        let lib_module_ref = format!("{}{}", lib_name, module.path());

        // See if the directory with that name exists
        if lib_root_path.exists() {
            if !lib_root_path.is_dir() {
                // It's a file and it exists, so just return the path
                let lib_path_url = Url::from_file_path(&lib_root_path)
                    .map_err(|_| format!("Could not create Url from '{:?}'", &lib_root_path))?;
                return Ok((lib_path_url.to_string(), Some(lib_module_ref)));
            }

            let provided_implementation_filename = lib_root_path.file_name()
                .chain_err(|| "Could not get library file name")?
                .to_str()
                .chain_err(|| "Could not convert library file name to a string")?;
            debug!("'{}' is a directory, so looking inside it for default file name '{}' or provided implementation file '{}' with extensions '{:?}'",
                   lib_root_path.display(), default_filename, provided_implementation_filename, extensions);
            for filename in [default_filename, provided_implementation_filename].iter() {
                let file = FileProvider::find_file(&lib_root_path, filename, extensions);
                if let Ok(file_path_as_url) = file {
                    return Ok((file_path_as_url, Some(lib_module_ref)));
                }
            }

            bail!("Found library folder '{}' in Library search path, but could not locate default file '{}' or provided implementation file '{}' within it with extensions '{:?}'",
                        lib_root_path.display(), default_filename, provided_implementation_filename, extensions)
        } else {
            // See if the file, with a .toml extension exists
            let mut implementation_path = lib_root_path.clone();
            implementation_path.set_extension("toml");
            if implementation_path.exists() {
                let lib_path_url = Url::from_file_path(&implementation_path)
                    .map_err(|_| format!("Could not create Url from '{:?}'", &implementation_path))?;
                return Ok((lib_path_url.to_string(), Some(lib_module_ref)));
            }
            bail!("Could not locate a folder called '{}' or an implementation file called '{}' in the Library search path ('FLOW_LIB_PATH' and '-L')",
                        lib_root_path.display(), implementation_path.display())
        }
    }

    // TODO
    fn resolve_url(lib_url: &Url, _lib_name: &str) -> Result<(String, Option<String>)> {
        bail!("Could not resolve library Url '{}' in library search path", lib_url)
    }
}

/*
    Urls for library flows and functions and values will be of the form:
        "lib://flowstdlib/stdio/stdout.toml"

    For a library Url, the only valid action is to resolve it to either an Http Url where the
    library is located, or a File path where the file is located, so only `resolve_url` is
    implemented.
*/
impl Provider for LibProvider {
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
    ///        "flowruntime/stdio/stdout/stdout"
    fn resolve_url(&self, url_str: &str, default_filename: &str, extensions: &[&str]) -> Result<(String, Option<String>)> {
        let url = Url::parse(url_str)
            .chain_err(|| format!("Could not convert '{}' to valid Url", url_str))?;
        let lib_name = url.host_str()
            .chain_err(|| format!("'lib_name' could not be extracted from the url '{}'", url))?;

        match self.lib_search_path.find(lib_name) {
            Ok(FoundType::File(mut lib_root_path)) => Self::resolve_file_path(&url, &mut lib_root_path, lib_name, default_filename, extensions),
            Ok(FoundType::Resource(lib_root_url)) => Self::resolve_url(&lib_root_url, lib_name),
            _ => bail!("Could not resolve library Url '{}' using library search path", url_str)
        }
    }

    // All Urls that start with "lib://" should resource to a different Url with "http(s)" or "file"
    // and so we should never get a request to get content from a Url with such a scheme
    fn get_contents(&self, _url: &str) -> Result<Vec<u8>> {
        bail!("We should never try to fetch contents from a lib: URL type, they should be resolved to file: or http|https:")
    }
}

#[cfg(test)]
mod test {
    use std::env;
    use std::path::Path;

    use simpath::Simpath;

    use super::LibProvider;
    use super::super::provider::Provider;

    fn set_lib_search_path() -> Simpath {
        let mut lib_search_path = Simpath::new("lib_search_path");
        let root_str = Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("Could not get project root dir");
        lib_search_path.add_directory(root_str.to_str().expect("Could not get root path as string"));
        println!("Lib search path set to '{}'", lib_search_path);
        lib_search_path
    }

    #[test]
    fn resolve_path() {
        let root_str = Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("Could not get project root dir");
        let provider: &dyn Provider = &LibProvider::new(set_lib_search_path());
        let lib_url = "lib://flowstdlib/control/tap";
        match provider.resolve_url(&lib_url, "", &["toml"]) {
            Ok((url, lib_ref)) => {
                assert_eq!(url, format!("file://{}/flowstdlib/control/tap/tap.toml", root_str.display().to_string()));
                assert_eq!(lib_ref, Some("flowstdlib/control/tap".to_string()));
            }
            Err(e) => panic!(e.to_string())
        }
    }
}

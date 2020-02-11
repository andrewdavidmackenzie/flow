use std::env;

use log::debug;
use simpath::Simpath;
use url::Url;

use flowrlib::errors::*;
use flowrlib::provider::Provider;

use crate::content::file_provider::FileProvider;

pub struct LibProvider;

/*
    Urls for library flows and functions and values will be of the form:
        "lib://flowstdlib/stdio/stdout.toml"

    Where 'flowstdlib' is the library name and 'stdio/stdout.toml' the path of the definition
    file within the library.

    For the lib provider, libraries maybe installed in multiple places in the file system.
    In order to find the content, a FLOW_LIB_PATH environment variable can be configured with a
    list of directories in which to look for the library in question.

    Once the library in question is found in the file system, then a "file:" Url is constructed
    that refers to the actual content, and this is returned.

    As the scheme of this Url is "file:" then a different content provider will be used to actually
    provide the content. Hence the "get" method for this provider is not imlemented and should
    never be called.
*/
impl Provider for LibProvider {
    /*
        Take the "lib:" Url (such as "lib://flowruntime/stdio/stdout") and extract the library
         name ("flowruntime")

        Using the "FLOW_LIB_PATH" environment variable attempt to locate the library's root folder
        in the file system.

        If located, then construct a PathBuf to refer to the definition file:
            - either "stdio/stdout.toml" or
            - "stdio/stdout/stdout.toml"

        within the library (using knowledge of library file structure).

        If the file exists, then create a "file:" Url that points to the file, for the file provider
        to use later to read the content.

        Also, construct a string that is a reference to that module in the library, such as:
            "flowruntime/stdio/stdout" and return that also.
    */
    fn resolve_url(&self, url_str: &str, default_filename: &str, _extensions: &[&str]) -> Result<(String, Option<String>)> {
        let url = Url::parse(url_str)
            .chain_err(|| format!("Could not convert '{}' to valid Url", url_str))?;
        let lib_name = url.host_str()
            .chain_err(|| format!("'lib_name' could not be extracted from host part of url '{}'", url))?;

        if let Err(_) = env::var("FLOW_LIB_PATH") {
            let parent_dir = std::env::current_dir().unwrap();
            debug!("Setting 'FLOW_LIB_PATH' to '{}'", parent_dir.to_string_lossy().to_string());
            env::set_var("FLOW_LIB_PATH", parent_dir.to_string_lossy().to_string());
        }

        let flow_lib_search_path = Simpath::new("FLOW_LIB_PATH");
        let mut lib_path = flow_lib_search_path.find(lib_name)
            .chain_err(|| format!("Could not find lib named '{}' in FLOW_LIB_PATH", lib_name))?;

        // Once we've foudn the (file) path where the library resides, append the rest of the
        // url path to it, to form a path to the directory where the process being loaded resides
        if url.path().len() > 0 {
            lib_path.push(&url.path()[1..]);
        }

        // Drop the file extension off the lib definition file path to get a lib reference
        let module = url.join("./").unwrap().join(lib_path.file_stem().unwrap().to_str().unwrap());
        let lib_ref = format!("{}{}", lib_name, module.unwrap().path());

        // See if the directory with that name exists
        if lib_path.exists() {
            if !lib_path.is_dir() {
                // It's a file and it exists, so just return the path
                let lib_path_url = Url::from_file_path(&lib_path)
                    .map_err(|_| format!("Could not create Url from '{:?}'", &lib_path))?;
                return Ok((lib_path_url.to_string(), Some(lib_ref.to_string())));
            }

            let provided_implementation_filename = lib_path.file_name().unwrap().to_str().unwrap();
            debug!("'{}' is a directory, so looking inside it for default file name '{}' or provided implemention file '{}' with extensions '{:?}'",
                   lib_path.display(), default_filename, provided_implementation_filename, _extensions);
            for filename in [default_filename, provided_implementation_filename].iter() {
                let file = FileProvider::find_file(&lib_path, filename, _extensions);
                if let Ok(file_path_as_url) = file {
                    return Ok(( file_path_as_url, Some(lib_ref.to_string())));
                }
            }

            bail!("Found library folder '{}' in 'FLOW_LIB_PATH', but could not locate default file '{}' or provided implementation file '{}' within it with extensions '{:?}'",
            lib_path.display(), default_filename, provided_implementation_filename, _extensions)
        } else {
            // See if the file, with a .toml extension exists
            let mut implementation_path = lib_path.clone();
            implementation_path.set_extension("toml");
            if implementation_path.exists() {
                let lib_path_url = Url::from_file_path(&implementation_path)
                    .map_err(|_| format!("Could not create Url from '{:?}'", &implementation_path))?;
                return Ok((lib_path_url.to_string(), Some(lib_ref.to_string())));
            }
            bail!("Could not locate a folder called '{}' or an implementation file called '{}' in 'FLOW_LIB_PATH'",
            lib_path.display(), implementation_path.display())
        }
    }

    // All Urls that start with "lib://" should resource to a different Url with "http(s)" or "file"
    // and so we should never get a request to get content from a Url with such a scheme
    fn get_contents(&self, _url: &str) -> Result<Vec<u8>> {
        unimplemented!();
    }
}

#[cfg(test)]
mod test {
    use std::env;

    use flowrlib::provider::Provider;

    use super::LibProvider;

    #[test]
    fn resolve_path() {
        let provider: &dyn Provider = &LibProvider;
        let mut root = env::current_dir().unwrap();
        root.pop();
        let root_str: String = root.as_os_str().to_str().unwrap().to_string();
        env::set_var("FLOW_LIB_PATH", &root_str);
        let lib_url = "lib://flowstdlib/control/tap";
        match provider.resolve_url(&lib_url, "".into(), &["toml"]) {
            Ok((url, lib_ref)) => {
                assert_eq!(url, format!("file://{}/flowstdlib/control/tap/tap.toml", root_str));
                assert_eq!(lib_ref, Some("flowstdlib/control/tap".to_string()));
            }
            Err(e) => assert!(false, e.to_string())
        }
    }
}

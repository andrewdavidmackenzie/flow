use std::env;

use flowrlib::provider::Provider;
use simpath::Simpath;
use url::Url;

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
        Take the "lib:" Url (such as "lib://flowstdlib/stdio/stdout.toml") and extract the library
         name ("flowstdlib")

        Using the "FLOW_LIB_PATH" environment variable attempt to locate the library's root folder
        in the file system.

        If located, then construct a PathBuf to refer to the specific file ("stdio/stdout.toml")
        within the library (using knowledge of library file structure).

        If the file exists, then create a "file:" Url that points to the file, for the file provider
        to use later to read the content.

        Also, construct a string that is a reference to that module in the library, such as:
            "flowstdlib/stdio/stdout" and return that also.
    */
    fn resolve(&self, url_str: &str, default_filename: &str) -> Result<(String, Option<String>), String> {
        let url = Url::parse(url_str)
            .map_err(|_| format!("Could not convert '{}' to valid Url", url_str))?;
        let lib_name = url.host_str().expect(
            &format!("'lib_name' could not be extracted from host part of url '{}'", url));

        if let Err(_) = env::var("FLOW_LIB_PATH") {
            let parent_dir = std::env::current_dir().unwrap();
            debug!("Setting 'FLOW_LIB_PATH' to '{}'", parent_dir.to_string_lossy().to_string());
            env::set_var("FLOW_LIB_PATH", parent_dir.to_string_lossy().to_string());
        }

        let flow_lib_search_path = Simpath::new("FLOW_LIB_PATH");
        let mut lib_path = flow_lib_search_path.find(lib_name)
            .map_err(|e| e.to_string())?;
        lib_path.push("src");
        lib_path.push(&url.path()[1..]); // Strip off leading '/' to concatenate to path

        // Drop the file extension off the lib definition file path to get a lib reference
        let module = url.join("./").
            unwrap().join(lib_path.file_stem().unwrap().to_str().unwrap());
        let lib_ref = format!("{}{}", lib_name, module.unwrap().path());

        if lib_path.exists() {
            if lib_path.is_dir() {
                debug!("'{:?}' is a directory, so looking for default file name '{}'", lib_path, default_filename);
                lib_path.push(default_filename);
                if !lib_path.exists() {
                    return Err(format!("Could not locate url '{}' in libraries in 'FLOW_LIB_PATH'", url));
                }
            }
            let lib_path_url = Url::from_file_path(&lib_path)
                .map_err(|_| format!("Could not create Url from '{:?}'", &lib_path))?;
            Ok((lib_path_url.to_string(), Some(lib_ref.to_string())))
        } else {
            Err(format!("Could not locate url '{}' in libraries in 'FLOW_LIB_PATH'", url))
        }
    }

    // All Urls that start with "lib://" should resource to a different Url with "http(s)" or "file"
    // and so we should never get a request to get content from a Url with such a scheme
    fn get(&self, _url: &str) -> Result<Vec<u8>, String> {
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
        let provider: &Provider = &LibProvider;
        let mut root = env::current_dir().unwrap();
        root.pop();
        let root_str: String = root.as_os_str().to_str().unwrap().to_string();
        env::set_var("FLOW_LIB_PATH", &root_str);
        let lib_url = "lib://flowstdlib/control/tap.toml";
        match provider.resolve(&lib_url, "".into()) {
            Ok((url, lib_ref)) => {
                assert_eq!(url, format!("file://{}/flowstdlib/src/control/tap.toml", root_str));
                assert_eq!(lib_ref, Some("flowstdlib/control/tap".to_string()));
            }
            Err(e) => assert!(false, e.to_string())
        }
    }
}

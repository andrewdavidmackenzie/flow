use url::Url;
use content::provider::Provider;
use simpath::Simpath;

pub struct LibProvider;

/*
    Urls for library flows and functions and values will be of the form:
        "lib://flowstdlib/src/stdio/stdout.toml"

    Where 'flowstdlib' is the library name and 'src/stdio/stdout.toml' the path of the definition
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

        Also, constuct a string that is a reference to that module in the library, such as:
            "flowstdlib/stdio/stdout" and return that also.
    */
    fn resolve(&self, url: &Url) -> Result<(Url, Option<String>), String> {
        let lib_name = url.host_str().unwrap();
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
            let resolved_url = Url::from_file_path(lib_path)
                .map_err(|_e| "Could not convert file path to Url".to_string())?;
            Ok((resolved_url, Some(lib_ref.to_string())))
        } else {
            Err(format!("Could not locate url '{}' in libraries in 'FLOW_LIB_PATH'", url))
        }
    }

    fn get(&self, _url: &Url) -> Result<String, String> {
        unimplemented!();
    }
}

#[cfg(test)]
mod test {
    use url::Url;
    use super::LibProvider;
    use content::provider::Provider;
    use std::env;

    #[test]
    fn resolve_path() {
        let provider: &Provider = &LibProvider;
        let mut root = env::current_dir().unwrap();
        root.pop();
        let root_str: String = root.as_os_str().to_str().unwrap().to_string();
        env::set_var("FLOW_LIB_PATH", &root_str);
        let lib_url = Url::parse("lib://flowstdlib/stdio/stdout.toml").unwrap();
        match provider.resolve(&lib_url) {
            Ok((url, lib_ref)) => {
                assert_eq!(url,
                           Url::parse(&format!("file://{}/flowstdlib/src/stdio/stdout.toml", root_str))
                               .unwrap());
                assert_eq!(lib_ref, Some("flowstdlib/stdio/stdout".to_string()));
            }
            Err(e) => assert!(false, e.to_string())
        }
    }
}

use flowrlib::provider::Provider;

pub struct LibProvider {
    flow_lib_path: String
}

impl LibProvider {
    pub fn new(flow_lib_path: String) -> Self {
        LibProvider {
            flow_lib_path
        }
    }
}

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
    fn resolve(&self, url_str: &str, _default_filename: &str) -> Result<(String, Option<String>), String> {
        let parts: Vec<&str> = url_str.split("://").collect();
        let path = parts[1];
        let parts: Vec<&str> = path.split("/").collect();

        let lib_name = parts[0];
        let mut lib_path = format!("{}/", self.flow_lib_path);
        lib_path.push_str(lib_name);
        lib_path.push_str("/");
        lib_path.push_str("src");

        for part in parts[1..].into_iter() {
            lib_path.push_str(&format!("/{}", part));
        }

        // TODO construct correctly the second one
        Ok((lib_path, Some(lib_name.to_string())))
    }

    // All Urls that start with "lib://" should resource to a different Url with "http(s)" or "file"
    // and so we should never get a request to get content from a Url with such a scheme
    fn get(&self, _url: &str) -> Result<Vec<u8>, String> {
        unimplemented!();
    }
}
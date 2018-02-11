use url::Url;
use content::provider::Provider;
use simpath::Simpath;

pub struct LibProvider;

/*
    Urls for library flows and functions and values will be of the form:
    source = "lib://flowstdlib/src/stdio/stdout.toml"
    Where 'flowstdlib' is the library name and 'src/stdio/stdout.toml' the path of the definition file
    within the library.

*/
impl Provider for LibProvider {
    /*
        For the lib provider, find the library file referenced in the Url in FLOW_LIB_PATH

        It should return a "file://" style Url, so the file provider will actually read it
    */
    fn resolve(&self, url: &Url) -> Result<(Url, Option<String>, Option<String>), String> {
        let lib_name = url.host_str().unwrap();
        let flow_lib_search_path = Simpath::new("FLOW_LIB_PATH");
        let mut lib_path = flow_lib_search_path.find(lib_name).map_err(|e| e.to_string())?;

        let lib_ref = &url.path()[1..];
        lib_path.push("src");
        lib_path.push(&lib_ref); // Strip off leading '/' to concatenate to path

        if lib_path.exists() {
            let resolved_url = Url::from_file_path(lib_path).map_err(|_e| "Could not convert file path to Url".to_string())?;
            Ok((resolved_url, Some(lib_name.to_string()), Some(lib_ref.to_string())))
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

    #[test]
    #[should_panic]
    fn get_contents_file_not_found() {
        let provider: &Provider = &LibProvider;
        provider.get(&Url::parse("lib:///no-such-file").unwrap()).unwrap();
    }
}

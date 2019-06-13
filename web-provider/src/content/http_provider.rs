use flowrlib::provider::Provider;
use url::Url;

pub struct HttpProvider;

impl Provider for HttpProvider {
    fn resolve(&self, url_str: &str, _default_filename: &str) -> Result<(String, Option<String>), String> {
        let url = Url::parse(url_str)
            .map_err(|_| format!("COuld not convert '{}' to valid Url", url_str))?;
        if url.path().ends_with('/') {
            info!("'{}' is a directory, so attempting to find context file in it", url);
            Ok((HttpProvider::find_default_file(&url).unwrap(), None))
        } else {
            Ok((url.to_string(), None))
        }
    }

    fn get(&self, _url: &str) -> Result<Vec<u8>, String> {
/*        let mut easy = Easy2::new(Collector(Vec::new()));
        easy.get(true).unwrap();
        easy.url(url).unwrap();
        easy.perform().unwrap();

        // TODO catch and return error string with details
        assert_eq!(easy.response_code().unwrap(), 200);
        let contents = easy.get_ref();
*/
        //Ok(contents.0.clone())
        Ok(Vec::from("hello"))
    }
}

impl HttpProvider {
    /*
        Passed a path to a directory, it searches for the first file it can find fitting the pattern
        "context.*", for known file extensions
    */
    fn find_default_file(_url: &Url) -> Result<String, String> {
        Err("Not implemented yet".to_string())
    }
}

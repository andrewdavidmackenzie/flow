use curl::easy::{Easy2, Handler, WriteError};
use flowrlib::provider::Provider;
use url::Url;

pub struct HttpProvider;

struct Collector(Vec<u8>);

impl Handler for Collector {
    fn write(&mut self, data: &[u8]) -> Result<usize, WriteError> {
        self.0.extend_from_slice(data);
        Ok(data.len())
    }
}

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

    fn get(&self, url: &str) -> Result<Vec<u8>, String> {
        let mut easy = Easy2::new(Collector(Vec::new()));
        easy.get(true).unwrap();
        easy.url(url).unwrap();
        easy.perform().unwrap();

        // TODO catch and return error string with details
        assert_eq!(easy.response_code().unwrap(), 200);
        let contents = easy.get_ref();
        Ok(contents.0.clone())
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

#[cfg(test)]
mod test {
    use flowrlib::provider::Provider;

    use super::HttpProvider;

    #[test]
    #[cfg_attr(not(feature = "online_tests"), ignore)]
    fn get_github_sample() {
        let provider: &Provider = &HttpProvider;
        provider.get("https://raw.githubusercontent.com/andrewdavidmackenzie/flow/master/samples/hello-world-simple/context.toml").unwrap();
    }

    #[test]
    #[should_panic]
    #[cfg_attr(not(feature = "online_tests"), ignore)]
    fn online_get_contents_file_not_found() {
        let provider: &Provider = &HttpProvider;
        provider.get("http://google.com/no-such-file").unwrap();
    }
}

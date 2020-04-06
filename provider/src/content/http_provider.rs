use curl::easy::{Easy2, Handler, WriteError};
use log::debug;
use log::info;
use url::Url;

use flowrlib::errors::*;
use flowrlib::provider::Provider;

pub struct HttpProvider;

struct Collector(Vec<u8>);

impl Handler for Collector {
    fn write(&mut self, data: &[u8]) -> std::result::Result<usize, WriteError> {
        self.0.extend_from_slice(data);
        Ok(data.len())
    }
}

impl Provider for HttpProvider {
    fn resolve_url(&self, url_str: &str, default_filename: &str, extensions: &[&str]) -> Result<(String, Option<String>)> {
        let url = Url::parse(url_str)
            .chain_err(|| format!("Could not convert '{}' to valid Url", url_str))?;
        if url.path().ends_with('/') {
            info!("'{}' is a directory, so attempting to find default file in it", url);
            Ok((HttpProvider::find_resource(url_str, default_filename, extensions)?, None))
        } else {
            Ok((url.to_string(), None))
        }
    }

    fn get_contents(&self, url: &str) -> Result<Vec<u8>> {
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
    pub fn find_resource(dir: &str, default_filename: &str, extensions: &[&str]) -> Result<String> {
        let mut resource = dir.to_string();
        resource.push_str(default_filename);

        Self::resource_by_extensions(&resource, extensions)
    }

    fn resource_by_extensions(resource: &str, extensions: &[&str]) -> Result<String> {
        // for that file path, try with all the allowed file extensions
        for extension in extensions {
            let resource_with_extension = format!("{}.{}", resource, extension);
            debug!("Looking for resource '{}'", resource_with_extension);

            let mut easy = Easy2::new(Collector(Vec::new()));
            easy.get(true).unwrap();
            easy.url(&resource_with_extension).unwrap();
            easy.perform().unwrap();

            if easy.response_code().unwrap() == 200 {
                return Ok(resource_with_extension);
            }
        }

        bail!("No resources found at path '{}' with any of these extensions '{:?}'", resource, extensions)
    }
}

#[cfg(test)]
mod test {
    use flowrlib::provider::Provider;

    use super::HttpProvider;

    #[test]
    fn resolve() {
        let provider = HttpProvider {};
        let folder_url = "https://raw.githubusercontent.com/andrewdavidmackenzie/flow/master/samples/hello-world/context.toml";
        let full_url = provider.resolve_url(folder_url, "context", &[&"toml"]).unwrap().0;
        assert_eq!(folder_url, &full_url);
    }

    #[test]
    #[cfg_attr(not(feature = "online_tests"), ignore)]
    fn resolve_default() {
        let provider = HttpProvider {};
        let folder_url = "https://raw.githubusercontent.com/andrewdavidmackenzie/flow/master/samples/hello-world/";
        let full_url = provider.resolve_url(folder_url, "context", &[&"toml"]).unwrap().0;
        let mut expected = folder_url.to_string();
        expected.push_str("context.toml");
        assert_eq!(full_url, expected);
    }

    #[test]
    #[cfg_attr(not(feature = "online_tests"), ignore)]
    fn get_github_sample() {
        let provider: &dyn Provider = &HttpProvider;
        provider.get_contents("https://raw.githubusercontent.com/andrewdavidmackenzie/flow/master/samples/hello-world/context.toml").unwrap();
    }

    #[test]
    #[should_panic]
    #[cfg_attr(not(feature = "online_tests"), ignore)]
    fn online_get_contents_file_not_found() {
        let provider: &dyn Provider = &HttpProvider;
        provider.get_contents("http://google.com/no-such-file").unwrap();
    }
}

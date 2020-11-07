use curl::easy::{Easy2, Handler, WriteError};
use log::debug;
use log::info;
use url::Url;

use crate::errors::*;

use super::provider::Provider;

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
        easy.get(true).map_err(|_| "Could not perform GET action on content source")?;
        easy.url(url).map_err(|_| "Could not set the url of the content source")?;
        easy.perform().map_err(|_| "Could not perform the action")?;
        match easy.response_code() {
            Ok(200) => {
                let contents = easy.get_ref();
                Ok(contents.0.clone())
            },
            _ => bail!("Did not get 200 response code from content source")
        }
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
            easy.get(true).chain_err(|| "Could not create curl GET request")?;
            easy.url(&resource_with_extension).chain_err(|| "Could not set Url on curl request")?;
            easy.perform().chain_err(|| "Could not perform curl request")?;

            if easy.response_code() == Ok(200) {
                return Ok(resource_with_extension);
            }
        }

        bail!("No resources found at path '{}' with any of these extensions '{:?}'", resource, extensions)
    }
}

#[cfg(test)]
mod test {
    use super::HttpProvider;
    use super::super::provider::Provider;

    #[test]
    fn resolve() {
        let provider = HttpProvider {};
        let folder_url = "https://raw.githubusercontent.com/andrewdavidmackenzie/flow/master/samples/hello-world/context.toml";
        let full_url = provider.resolve_url(folder_url, "context", &[&"toml"]).unwrap().0;
        assert_eq!(folder_url, &full_url);
    }

    #[test]
    fn resolve_default() {
        let provider = HttpProvider {};
        let folder_url = "https://raw.githubusercontent.com/andrewdavidmackenzie/flow/master/samples/hello-world/";
        let full_url = provider.resolve_url(folder_url, "context", &[&"toml"]).unwrap().0;
        let mut expected = folder_url.to_string();
        expected.push_str("context.toml");
        assert_eq!(full_url, expected);
    }

    #[test]
    fn get_github_sample() {
        let provider: &dyn Provider = &HttpProvider;
        let found = provider.get_contents("https://raw.githubusercontent.com/andrewdavidmackenzie/flow/master/samples/hello-world/context.toml");
        assert!(found.is_ok());
    }

    #[test]
    fn online_get_contents_file_not_found() {
        let provider: &dyn Provider = &HttpProvider;
        let not_found = provider.get_contents("http://foo.com/no-such-file");
        assert!(not_found.is_err());
    }
}

use curl::easy::{Easy2, Handler, WriteError};
use log::debug;
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
        // Try the url directly first
        if Self::resource_exists(url_str).is_ok() {
            return Ok((url_str.into(), None));
        }

        // Attempting to find default file under this path
        if let Ok(found) = Self::resource_by_extensions(&format!("{}/{}", url_str, default_filename), extensions) {
            return Ok((found, None));
        }

        // Attempt to find file with same name as final segment under that path
        let url = Url::parse(url_str).map_err(|e| e.to_string())?;
        let mut segments = url.path_segments().ok_or("Could not get path segments")?;
        let file_name = segments.next_back().ok_or("Could not get last path segment")?;
        if let Ok(found) = Self::resource_by_extensions(&format!("{}/{}", url_str, file_name), extensions) {
            return Ok((found, None));
        }

        bail!("Could not resolve the Url: '{}'", url_str)
    }

    fn get_contents(&self, url_str: &str) -> Result<Vec<u8>> {
        let mut easy = Easy2::new(Collector(Vec::new()));
        easy.get(true).map_err(|_| "Could not perform GET action on content source")?;
        easy.url(url_str).map_err(|_| "Could not set the url of the content source")?;
        easy.perform().map_err(|_| "Could not perform the action")?;
        match easy.response_code().map_err(|e| e.to_string())? {
            200..=299 => {
                let contents = easy.get_ref();
                Ok(contents.0.clone())
            },
            error_code => bail!("Response code: {} from '{}'", error_code, url_str)
        }
    }
}

impl HttpProvider {
    fn resource_exists(url_str: &str) -> Result<()> {
        debug!("Looking for resource '{}'", url_str);
        let mut easy = Easy2::new(Collector(Vec::new()));
        easy.nobody(true).map_err(|e| e.to_string())?;

        easy.url(url_str).map_err(|e| e.to_string())?;
        easy.perform().map_err(|e| e.to_string())?;
        let response_code = easy.response_code().map_err(|e| e.to_string())?;

        // Consider 301 - Permanently Moved as the resource NOT being at this Url
        // An option to consider is asking the request library to follow the redirect.
        match response_code {
            200..=299 => Ok(()),
            error_code => bail!("Response code: {} from '{}'", error_code, url_str)
        }
    }

    fn resource_by_extensions(resource: &str, extensions: &[&str]) -> Result<String> {
        // for that file path, try with all the allowed file extensions
        for extension in extensions {
            let resource_with_extension = format!("{}.{}", resource, extension);
            if Self::resource_exists(&resource_with_extension).is_ok() {
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
        let full_url = provider.resolve_url(folder_url, "context", &[&"toml"])
            .expect("Could not resolve url").0;
        assert_eq!(folder_url, &full_url);
    }

    #[test]
    fn resolve_default() {
        let provider = HttpProvider {};
        let folder_url = "https://raw.githubusercontent.com/andrewdavidmackenzie/flow/master/samples/hello-world";
        let full_url = provider.resolve_url(folder_url, "context", &[&"toml"])
            .expect("Could not resolve url").0;
        let mut expected = folder_url.to_string();
        expected.push_str("/context.toml");
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

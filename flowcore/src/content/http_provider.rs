use curl::easy::{Easy2, Handler, WriteError};
use log::debug;
use url::Url;

use crate::errors::*;
use crate::lib_provider::Provider;

pub struct HttpProvider;

struct Collector(Vec<u8>);

impl Handler for Collector {
    fn write(&mut self, data: &[u8]) -> std::result::Result<usize, WriteError> {
        self.0.extend_from_slice(data);
        Ok(data.len())
    }
}

impl Provider for HttpProvider {
    fn resolve_url(
        &self,
        url: &Url,
        default_filename: &str,
        extensions: &[&str],
    ) -> Result<(Url, Option<String>)> {
        // Try the url directly first
        if Self::resource_exists(url).is_ok() {
            return Ok((url.clone(), None));
        }

        // Try the url with possible extensions next
        if let Ok(found) = Self::resource_by_extensions(url, extensions) {
            return Ok((found, None));
        }

        // Attempting to find default file under this path, with any of the valid extensions
        let default_filename_url = Url::parse(&format!("{}/{}", url.to_string(), default_filename))
            .chain_err(|| "Could not append default_filename to url")?;
        if let Ok(found) = Self::resource_by_extensions(&default_filename_url, extensions) {
            return Ok((found, None));
        }

        // Attempt to find file with same name as final segment under that path, with any extension
        let mut segments = url.path_segments().ok_or("Could not get path segments")?;
        let file_name = segments
            .next_back()
            .ok_or("Could not get last path segment")?;
        let filename_url = Url::parse(&format!("{}/{}", url.to_string(), file_name))
            .chain_err(|| "Could not append filename after directory in Url")?;
        if let Ok(found) = Self::resource_by_extensions(&filename_url, extensions) {
            return Ok((found, None));
        }

        bail!("Could not resolve the Url: '{}'", url)
    }

    fn get_contents(&self, url: &Url) -> Result<Vec<u8>> {
        let mut easy = Easy2::new(Collector(Vec::new()));
        easy.get(true).chain_err(|| "Could not set GET operation")?;
        easy.url(url.as_str()).chain_err(|| "Could not set Url")?;
        easy.perform().chain_err(|| "Could not perform operation")?;
        match easy
            .response_code()
            .chain_err(|| "Could not get status code")?
        {
            200..=299 => {
                let contents = easy.get_ref();
                Ok(contents.0.clone())
            }
            error_code => bail!("Response code: {} from '{}'", error_code, url.as_str()),
        }
    }
}

impl HttpProvider {
    fn resource_exists(url: &Url) -> Result<()> {
        debug!("Looking for resource '{}'", url);
        let mut easy = Easy2::new(Collector(Vec::new()));
        easy.nobody(true)
            .chain_err(|| "Could not set NO_BODY on operation")?;

        easy.url(url.as_str())
            .chain_err(|| "Could not set Url on operation")?;
        easy.perform().chain_err(|| "Could not perform operation")?;

        // Consider 301 - Permanently Moved as the resource NOT being at this Url
        // An option to consider is asking the request library to follow the redirect.
        match easy
            .response_code()
            .chain_err(|| "Could not get status code")?
        {
            200..=299 => Ok(()),
            error_code => bail!("Response code: {} from '{}'", error_code, url.as_str()),
        }
    }

    fn resource_by_extensions(resource: &Url, extensions: &[&str]) -> Result<Url> {
        // for that file path, try with all the allowed file extensions
        for extension in extensions {
            let resource_with_extension =
                Url::parse(&format!("{}.{}", resource.as_str(), extension))
                    .chain_err(|| "Could not parse Url with extension added")?;
            if Self::resource_exists(&resource_with_extension).is_ok() {
                return Ok(resource_with_extension);
            }
        }

        bail!(
            "No resources found at path '{}' with any of these extensions '{:?}'",
            resource,
            extensions
        )
    }
}

#[cfg(test)]
mod test {
    use url::Url;

    use crate::lib_provider::Provider;

    use super::HttpProvider;

    #[test]
    #[cfg(feature = "online_tests")]
    fn resolve_web() {
        let provider = HttpProvider {};
        let folder_url = Url::parse("https://raw.githubusercontent.com/andrewdavidmackenzie/flow/master/samples/hello-world/context.toml")
            .expect("Could not form Url");
        let full_url = provider
            .resolve_url(&folder_url, "context", &[&"toml"])
            .expect("Could not resolve url");
        assert_eq!(folder_url.as_str(), full_url.as_str());
    }

    #[test]
    #[cfg(feature = "online_tests")]
    fn resolve_default_web() {
        let provider = HttpProvider {};
        let folder_url = Url::parse("https://raw.githubusercontent.com/andrewdavidmackenzie/flow/master/samples/hello-world")
            .expect("Could not form Url");
        let expected_url = Url::parse("https://raw.githubusercontent.com/andrewdavidmackenzie/flow/master/samples/hello-world/context.toml")
            .expect("Could not form Url");
        let full_url = provider
            .resolve_url(&folder_url, "context", &[&"toml"])
            .expect("Could not resolve url");
        assert_eq!(full_url.as_str(), expected_url.as_str());
    }

    #[test]
    #[cfg(feature = "online_tests")]
    fn get_github_sample() {
        let provider: &dyn Provider = &HttpProvider;
        let url = Url::parse("https://raw.githubusercontent.com/andrewdavidmackenzie/flow/master/samples/hello-world/context.toml")
            .expect("Could not form Url");
        let found = provider.get_contents(&url);
        assert!(found.is_ok());
    }

    #[test]
    fn online_get_contents_file_not_found() {
        let provider: &dyn Provider = &HttpProvider;
        let url = Url::parse("http://foo.com/no-such-file").expect("Could not form Url");
        let not_found = provider.get_contents(&url);
        assert!(not_found.is_err());
    }
}

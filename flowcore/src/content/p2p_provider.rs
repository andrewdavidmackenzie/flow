use curl::easy::{Handler, WriteError};
use url::Url;

use crate::errors::*;
use crate::provider::Provider;

/// The `P2pProvider` implements the `Provider` trait and takes care of fetching content over
/// the p2p network of flow peers
pub struct P2pProvider;

struct Collector(Vec<u8>);

impl Handler for Collector {
    fn write(&mut self, data: &[u8]) -> std::result::Result<usize, WriteError> {
        self.0.extend_from_slice(data);
        Ok(data.len())
    }
}

impl Provider for P2pProvider {
    fn resolve_url(
        &self,
        _url: &Url,
        _default_filename: &str,
        _extensions: &[&str],
    ) -> Result<(Url, Option<Url>)> {
        unimplemented!();
    }

    fn get_contents(&self, _url: &Url) -> Result<Vec<u8>> {
        unimplemented!();
    }
}

impl P2pProvider {
    #[allow(clippy::new_without_default)]
    /// Create a new instance of a p2p content provider
    pub fn new() -> Self {
        P2pProvider
    }
}

#[cfg(test)]
mod test {
}

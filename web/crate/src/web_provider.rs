use flowrlib::provider::Provider;
use std::convert::Into;

pub struct WebProvider {
}

impl Provider for WebProvider {
    fn resolve(&self, source: &str, _default_filename: &str) -> Result<(String, Option<String>), String> {
        Ok((source.to_string(), None))
    }

    fn get(&self, _url: &str) -> Result<Vec<u8>, String> {
        Ok("".into())
    }
}
use std::fs::File;
use std::io::Read;

use flowrlib::loader::Provider;

pub struct FlowrProvider {}

impl Provider for FlowrProvider {
    fn get_content<'a>(&self, path: &str) -> Result<String, String> {
        let mut file = File::open(path).map_err(|e| e.to_string())?;
        let mut contents = String::new();
        file.read_to_string(&mut contents).map_err(|e| e.to_string())?;
        Ok(contents)
    }
}
use crate::compiler::loader::Deserializer;

use super::json_deserializer::FlowJsonLoader;
use super::toml_deserializer::FlowTomelLoader;
use super::yaml_deserializer::FlowYamlLoader;

const TOML: &dyn Deserializer = &FlowTomelLoader as &dyn Deserializer;
const YAML: &dyn Deserializer = &FlowYamlLoader as &dyn Deserializer;
const JSON: &dyn Deserializer = &FlowJsonLoader as &dyn Deserializer;

const ACCEPTED_EXTENSIONS: [&str; 4] = ["toml", "yaml", "json", "yml"];

pub fn get_deserializer(url: &str) -> Result<&'static dyn Deserializer, String> {
    match get_file_extension(url) {
        Some(ext) => {
            match ext.as_ref() {
                "toml" => Ok(TOML),
                "yaml" | "yml" => Ok(YAML),
                "json" => Ok(JSON),
                _ => Err("Unknown file extension so cannot determine which deserializer to use".to_string())
            }
        }
        None => Err("No file extension so cannot determine which deserializer to use".to_string())
    }
}

pub fn get_accepted_extensions() -> &'static [&'static str] {
    &ACCEPTED_EXTENSIONS
}

pub fn get_file_extension(url: &str) -> Option<String> {
    let segments = url.split('/');
    if let Some(last_segment) = segments.last() {
        let splits: Vec<&str> = last_segment.split('.').collect();
        if splits.len() >= 2 {
            return Some((*splits.last().unwrap()).to_string());
        }
    }
    None
}

#[cfg(test)]
mod test {
    use super::get_accepted_extensions;
    use super::get_deserializer;
    use super::get_file_extension;

    #[test]
    fn get_accepted_extension_test() {
        let accepted = get_accepted_extensions();

        assert!(accepted.contains(&"toml"));
        assert!(accepted.contains(&"json"));
        assert!(accepted.contains(&"yaml"));
        assert!(accepted.contains(&"yml"));
    }

    #[test]
    fn no_extension() {
        assert!(get_file_extension("file:///no_extension").is_none(),
                "No file extension should not find a deserializer");
    }

    #[test]
    fn valid_file_extension() {
        get_file_extension("file::///OK.toml").unwrap();
    }

    #[test]
    fn valid_http_extension() {
        get_file_extension("http://test.com/OK.toml").unwrap();
    }

    #[test]
    fn invalid_extension() {
        assert!(get_deserializer("file:///extension.wrong").is_err(),
                "Unknown file extension should not find a deserializer");
    }

    #[test]
    fn toml_extension_loader() {
        get_deserializer("file:///extension.toml").unwrap();
    }

    #[test]
    fn yaml_extension_loader() {
        get_deserializer("file:///extension.yaml").unwrap();
    }
}
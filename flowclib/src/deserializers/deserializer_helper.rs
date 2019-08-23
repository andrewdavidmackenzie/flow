use super::yaml_deserializer::FlowYamlLoader;
use super::toml_deserializer::FlowTomelLoader;
use super::json_deserializer::FlowJsonLoader;
use crate::compiler::loader::Deserializer;

const TOML: &dyn Deserializer = &FlowTomelLoader as &dyn Deserializer;
const YAML: &dyn Deserializer = &FlowYamlLoader as &dyn Deserializer;
const JSON: &dyn Deserializer = &FlowJsonLoader as &dyn Deserializer;

pub fn get_deserializer(url: &str) -> Result<&'static dyn Deserializer, String> {
    match get_file_extension(url) {
        Ok(ext) => {
            match ext.as_ref() {
                "toml" => Ok(TOML),
                "yaml" | "yml" => Ok(YAML),
                "json" => Ok(JSON),
                _ => Err("Unknown file extension so cannot determine which loader to use".to_string())
            }
        }
        Err(e) => Err(format!("Cannot determine which loader to use ({})", e.to_string())
        )
    }
}

fn get_file_extension(url: &str) -> Result<String, String> {
    let segments = url.split("/");
    let last_segment = segments.last().ok_or_else(|| "no segments")?;
    let splits: Vec<&str> = last_segment.split('.').collect();
    if splits.len() < 2 {
        Err("No file extension".to_string())
    } else {
        Ok(splits.last().unwrap().to_string())
    }
}

#[cfg(test)]
mod test {
    use super::get_file_extension;
    use super::get_deserializer;

    #[test]
    fn no_extension() {
        assert!(get_file_extension("file:///no_extension").is_err(),
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
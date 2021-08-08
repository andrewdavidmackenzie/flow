use url::Url;

use crate::compiler::loader::Deserializer;
use crate::model::process::Process;

use super::json_deserializer::JsonDeserializer;
use super::toml_deserializer::TomlDeserializer;
use super::yaml_deserializer::YamlDeserializer;

/// Return a Deserializer based on the file extension of the resource referred to from `url` input
pub fn get_deserializer(url: &Url) -> Result<&dyn Deserializer<Process>, String> {
    match get_file_extension(url) {
        Some(ext) => match ext {
            "toml" => Ok(&TomlDeserializer),
            "yaml" | "yml" => Ok(&YamlDeserializer),
            "json" => Ok(&JsonDeserializer),
            _ => Err(
                "Unknown file extension so cannot determine which deserializer to use".to_string(),
            ),
        },
        None => Err("No file extension so cannot determine which deserializer to use".to_string()),
    }
}

/// Get the file extension of the resource referred to by `url`
fn get_file_extension(url: &Url) -> Option<&str> {
    url.path_segments()?.last()?.rsplit_once('.').map(|t| t.1)
}

#[cfg(test)]
mod test {
    use url::Url;

    use super::get_deserializer;
    use super::get_file_extension;

    #[test]
    fn no_extension() {
        let url = &Url::parse("file:///no_extension").expect("Could not create Url");
        let ext = get_file_extension(&url);
        assert!(
            ext.is_none(),
            "should not find a file extension in filename 'no_extension'"
        );
    }

    #[test]
    fn valid_file_extension() {
        assert_eq!(
            get_file_extension(&Url::parse("file::///filename.toml").expect("Could not parse Url")),
            Some("toml")
        );
    }

    #[test]
    fn valid_http_extension() {
        assert_eq!(
            get_file_extension(
                &Url::parse("http://test.com/filename.toml").expect("Could not create Url")
            ),
            Some("toml")
        );
    }

    #[test]
    fn invalid_extension() {
        assert!(
            get_deserializer(&Url::parse("file:///extension.wrong").expect("Could not create Url"))
                .is_err(),
            "Unknown file extension should not find a deserializer"
        );
    }

    #[test]
    fn toml_extension_loader() {
        assert_eq!(
            get_deserializer(&Url::parse("file:///filename.toml").expect("Could not create Url"))
                .expect("Could not get a deserializer")
                .name(),
            "Toml"
        );
    }

    #[test]
    fn yaml_extension_loader() {
        assert_eq!(
            get_deserializer(&Url::parse("file:///filename.yaml").expect("Could not create Url"))
                .expect("Could not get a deserializer")
                .name(),
            "Yaml"
        );
    }

    #[test]
    fn yml_extension_loader() {
        assert_eq!(
            get_deserializer(&Url::parse("file:///filename.yml").expect("Could not create Url"))
                .expect("Could not get a deserializer")
                .name(),
            "Yaml"
        );
    }

    #[test]
    fn json_extension_loader() {
        assert_eq!(
            get_deserializer(&Url::parse("file:///filename.json").expect("Could not create Url"))
                .expect("Could not get a deserializer")
                .name(),
            "Json"
        );
    }
}

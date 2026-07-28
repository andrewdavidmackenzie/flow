use serde::de::DeserializeOwned;
use url::Url;

use crate::bail;
use crate::errors::{Result, ResultExt};

/// Deserialize `contents` loaded from `url` into type `T`, selecting the format
/// (TOML, JSON, or YAML) based on the file extension of `url`.
///
/// # Errors
///
/// Returns `Err` if the file extension is missing or unrecognized, or if
/// deserialization fails.
pub fn deserialize<T>(url: &Url, contents: &str) -> Result<T>
where
    T: DeserializeOwned + 'static,
{
    match get_file_extension(url) {
        Some("toml") => {
            toml::from_str(contents).chain_err(|| format!("Error deserializing Toml from: '{url}'"))
        }
        Some("yaml" | "yml") => serde_yml::from_str(contents)
            .chain_err(|| format!("Error deserializing Yaml from: '{url}'")),
        Some("json") => serde_json::from_str(contents)
            .chain_err(|| format!("Error deserializing Json from: '{url}'")),
        Some(_) => bail!("Unknown file extension so cannot determine which deserializer to use"),
        None => bail!("No file extension so cannot determine which deserializer to use"),
    }
}

/// Return the name of the deserializer format that would be used for `url`,
/// based on its file extension.
///
/// # Errors
///
/// Returns `Err` if the file extension is missing or unrecognized.
pub fn format_name(url: &Url) -> Result<&'static str> {
    match get_file_extension(url) {
        Some("toml") => Ok("Toml"),
        Some("yaml" | "yml") => Ok("Yaml"),
        Some("json") => Ok("Json"),
        Some(_) => bail!("Unknown file extension"),
        None => bail!("No file extension"),
    }
}

/// Get the file extension of the resource referred to by `url`
fn get_file_extension(url: &Url) -> Option<&str> {
    url.path_segments()?
        .next_back()?
        .rsplit_once('.')
        .map(|t| t.1)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {
    use serde_derive::{Deserialize, Serialize};
    use url::Url;

    use super::{deserialize, format_name, get_file_extension};

    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
    struct TestStruct {
        name: String,
    }

    #[test]
    fn no_extension() {
        let url = &Url::parse("file:///no_extension").expect("Could not create Url");
        let ext = get_file_extension(url);
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
        let url = Url::parse("file:///extension.wrong").expect("Could not create Url");
        assert!(
            deserialize::<TestStruct>(&url, "").is_err(),
            "Unknown file extension should not find a deserializer"
        );
    }

    #[test]
    fn toml_format_name() {
        let url = Url::parse("file:///filename.toml").expect("Could not create Url");
        assert_eq!(
            format_name(&url).expect("Could not get format name"),
            "Toml"
        );
    }

    #[test]
    fn yaml_format_name() {
        let url = Url::parse("file:///filename.yaml").expect("Could not create Url");
        assert_eq!(
            format_name(&url).expect("Could not get format name"),
            "Yaml"
        );
    }

    #[test]
    fn yml_format_name() {
        let url = Url::parse("file:///filename.yml").expect("Could not create Url");
        assert_eq!(
            format_name(&url).expect("Could not get format name"),
            "Yaml"
        );
    }

    #[test]
    fn json_format_name() {
        let url = Url::parse("file:///filename.json").expect("Could not create Url");
        assert_eq!(
            format_name(&url).expect("Could not get format name"),
            "Json"
        );
    }

    #[test]
    fn deserialize_toml() {
        let url = Url::parse("file:///test.toml").expect("Could not create Url");
        let result: TestStruct =
            deserialize(&url, "name = \"hello\"").expect("Could not deserialize");
        assert_eq!(result.name, "hello");
    }

    #[test]
    fn deserialize_json() {
        let url = Url::parse("file:///test.json").expect("Could not create Url");
        let result: TestStruct =
            deserialize(&url, r#"{"name": "hello"}"#).expect("Could not deserialize");
        assert_eq!(result.name, "hello");
    }

    #[test]
    fn deserialize_yaml() {
        let url = Url::parse("file:///test.yaml").expect("Could not create Url");
        let result: TestStruct = deserialize(&url, "name: hello").expect("Could not deserialize");
        assert_eq!(result.name, "hello");
    }

    #[test]
    fn invalid_toml_content() {
        let url = Url::parse("file:///test.toml").expect("Could not create Url");
        assert!(deserialize::<TestStruct>(&url, "{invalid").is_err());
    }

    #[test]
    fn invalid_json_content() {
        let url = Url::parse("file:///test.json").expect("Could not create Url");
        assert!(deserialize::<TestStruct>(&url, "{invalid").is_err());
    }
}

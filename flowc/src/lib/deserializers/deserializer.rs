use serde::de::DeserializeOwned;
use serde::Deserialize;
use url::Url;

use crate::errors::*;

use super::json_deserializer::JsonDeserializer;
use super::toml_deserializer::TomlDeserializer;
use super::yaml_deserializer::YamlDeserializer;

/// All deserializers have to implement this trait for content deserialization, plus a method
/// to return their name to be able to inform the user of which deserializer was used
pub trait Deserializer<'a, T: Deserialize<'a>> {
    /// Deserialize the supplied `content` that was loaded from `url` into a `P`
    fn deserialize(&self, contents: &'a str, url: Option<&Url>) -> Result<T>;
    /// Return the name of the serializer implementing this trait
    fn name(&self) -> &str;
}

/// Return a Deserializer based on the file extension of the resource referred to from `url` input
pub fn get_deserializer<'a, T>(url: &'a Url) -> Result<Box<dyn Deserializer<'a, T> + 'a>>
where
    T: DeserializeOwned + 'static,
{
    match get_file_extension(url) {
        Some(ext) => match ext {
            "toml" => Ok(Box::new(TomlDeserializer::new())),
            "yaml" | "yml" => Ok(Box::new(YamlDeserializer::new())),
            "json" => Ok(Box::new(JsonDeserializer::new())),
            _ => bail!("Unknown file extension so cannot determine which deserializer to use"),
        },
        None => bail!("No file extension so cannot determine which deserializer to use"),
    }
}

/// Get the file extension of the resource referred to by `url`
fn get_file_extension(url: &Url) -> Option<&str> {
    url.path_segments()?.last()?.rsplit_once('.').map(|t| t.1)
}

#[cfg(test)]
mod test {
    use url::Url;

    use crate::model::process::Process;

    use super::get_deserializer;
    use super::get_file_extension;

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
        assert!(
            get_deserializer::<Process>(
                &Url::parse("file:///extension.wrong").expect("Could not create Url")
            )
            .is_err(),
            "Unknown file extension should not find a deserializer"
        );
    }

    #[test]
    fn toml_extension_loader() {
        assert_eq!(
            get_deserializer::<Process>(
                &Url::parse("file:///filename.toml").expect("Could not create Url")
            )
            .expect("Could not get a deserializer")
            .name(),
            "Toml"
        );
    }

    #[test]
    fn yaml_extension_loader() {
        assert_eq!(
            get_deserializer::<Process>(
                &Url::parse("file:///filename.yaml").expect("Could not create Url")
            )
            .expect("Could not get a deserializer")
            .name(),
            "Yaml"
        );
    }

    #[test]
    fn yml_extension_loader() {
        assert_eq!(
            get_deserializer::<Process>(
                &Url::parse("file:///filename.yml").expect("Could not create Url")
            )
            .expect("Could not get a deserializer")
            .name(),
            "Yaml"
        );
    }

    #[test]
    fn json_extension_loader() {
        assert_eq!(
            get_deserializer::<Process>(
                &Url::parse("file:///filename.json").expect("Could not create Url")
            )
            .expect("Could not get a deserializer")
            .name(),
            "Json"
        );
    }
}

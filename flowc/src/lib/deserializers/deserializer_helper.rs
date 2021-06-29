use url::Url;

use crate::compiler::loader::Deserializer;

use super::json_deserializer::FlowJsonLoader;
use super::toml_deserializer::FlowTomlLoader;
use super::yaml_deserializer::FlowYamlLoader;

const TOML: &dyn Deserializer = &FlowTomlLoader as &dyn Deserializer;
const YAML: &dyn Deserializer = &FlowYamlLoader as &dyn Deserializer;
const JSON: &dyn Deserializer = &FlowJsonLoader as &dyn Deserializer;

const ACCEPTED_EXTENSIONS: [&str; 4] = ["toml", "yaml", "json", "yml"];

/// Return a Deserializer based on the file extension of the file referred to from `url` input
pub fn get_deserializer(url: &Url) -> Result<&'static dyn Deserializer, String> {
    match get_file_extension(url) {
        Some(ext) => match ext {
            "toml" => Ok(TOML),
            "yaml" | "yml" => Ok(YAML),
            "json" => Ok(JSON),
            _ => Err(
                "Unknown file extension so cannot determine which deserializer to use".to_string(),
            ),
        },
        None => Err("No file extension so cannot determine which deserializer to use".to_string()),
    }
}

/// Return an array of the file extensions we have deserializers able to deserialize
pub fn get_accepted_extensions() -> &'static [&'static str] {
    &ACCEPTED_EXTENSIONS
}

/// Get the file extension of the resource referred to by `url`
pub fn get_file_extension(url: &Url) -> Option<&str> {
    let last_segment = url.path_segments()?.last()?;
    // Split returns one element if there is no occurrence of pattern in the string,
    // in which case we want to return None
    // if there are 2 or more elements returned by split, then at least 1 "." occurred and possible
    // more. So return the string after the last "." as the extension
    let splits: Vec<&str> = last_segment.split('.').collect();
    if splits.len() > 1 {
        return splits.into_iter().last();
    }
    None
}

#[cfg(test)]
mod test {
    use url::Url;

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
        let url = &Url::parse("file:///no_extension").expect("Could not create Url");
        let ext = get_file_extension(&url);
        assert!(
            ext.is_none(),
            "should not find a file extension in filename 'no_extension'"
        );
    }

    #[test]
    fn valid_file_extension() {
        get_file_extension(&Url::parse("file::///OK.toml").expect("Could not create Url")).unwrap();
    }

    #[test]
    fn valid_http_extension() {
        get_file_extension(&Url::parse("http://test.com/OK.toml").expect("Could not create Url"))
            .unwrap();
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
        get_deserializer(&Url::parse("file:///extension.toml").expect("Could not create Url"))
            .unwrap();
    }

    #[test]
    fn yaml_extension_loader() {
        get_deserializer(&Url::parse("file:///extension.yaml").expect("Could not create Url"))
            .unwrap();
    }
}

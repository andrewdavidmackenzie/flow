use std::marker::PhantomData;

use serde::de::DeserializeOwned;
use url::Url;

use crate::errors::*;

use super::deserializer::Deserializer;

/// Struct representing a Generic deserializer of content stored in Yaml format
#[derive(Default)]
pub struct YamlDeserializer<T>
where
    T: DeserializeOwned,
{
    t: PhantomData<T>,
}

impl<T> YamlDeserializer<T>
where
    T: DeserializeOwned,
{
    /// Create a new YamlDeserializer
    pub fn new() -> Self {
        YamlDeserializer { t: PhantomData }
    }
}

impl<'a, T> Deserializer<'a, T> for YamlDeserializer<T>
where
    T: DeserializeOwned,
{
    fn deserialize(&self, contents: &'a str, url: Option<&Url>) -> Result<T> {
        serde_yaml::from_str(contents).chain_err(|| {
            format!(
                "Error deserializing Yaml from: '{}'",
                url.map_or("URL was None".to_owned(), |u| u.to_string())
            )
        })
    }

    fn name(&self) -> &str {
        "Yaml"
    }
}

#[cfg(test)]
mod test {
    use serde_derive::{Deserialize, Serialize};
    use serde_yaml::Error;

    use crate::model::metadata::MetaData;

    use super::super::deserializer::Deserializer;
    use super::YamlDeserializer;

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct TestStruct {
        name: String,
    }

    #[test]
    fn invalid_yaml() {
        let deserializer = YamlDeserializer::<TestStruct>::new();

        assert!(
            deserializer.deserialize("{}", None).is_err(),
            "Should not have parsed correctly as is invalid Yaml"
        );
    }

    #[test]
    fn flow() {
        let flow_with_name = "
name: 'hello-world-simple-toml'
";

        let deserializer = YamlDeserializer::<TestStruct>::new();

        assert!(
            deserializer.deserialize(flow_with_name, None).is_ok(),
            "Did not parse correctly but is valid Yaml"
        );
    }

    #[test]
    fn metadata() {
        let metadata = "\
name: \"me\"
version: \"1.1.1\"
description: \"ok\"
authors: [\"Andrew <andrew@foo.com>\"]
    ";

        let result: Result<MetaData, Error> = serde_yaml::from_str(metadata);
        match result {
            Ok(md) => {
                assert_eq!(md.name, "me".to_string());
                assert_eq!(md.version, "1.1.1".to_string());
                assert_eq!(md.description, "ok".to_string());
                assert_eq!(md.authors, vec!("Andrew <andrew@foo.com>".to_string()));
            }
            Err(e) => panic!("Deserialization error: {:?}", e),
        }
    }
}

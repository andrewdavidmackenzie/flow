use std::marker::PhantomData;

use serde::Deserialize;
use url::Url;

use crate::errors::*;

use super::deserializer::Deserializer;

/// Struct representing a Generic deserializer of content stored in Toml format
#[derive(Default)]
pub struct TomlDeserializer<'a, T>
where
    T: Deserialize<'a>,
{
    t: PhantomData<&'a T>,
}

impl<'a, T> TomlDeserializer<'a, T>
where
    T: Deserialize<'a>,
{
    /// Create a new TomlDeserializer
    pub fn new() -> Self {
        TomlDeserializer { t: PhantomData }
    }
}

impl<'a, T> Deserializer<'a, T> for TomlDeserializer<'a, T>
where
    T: Deserialize<'a>,
{
    fn deserialize(&self, contents: &'a str, url: Option<&Url>) -> Result<T> {
        toml::from_str(contents).chain_err(|| {
            format!(
                "Error deserializing Toml from: '{}'",
                url.map_or("URL was None".to_owned(), |u| u.to_string())
            )
        })
    }

    fn name(&self) -> &str {
        "Toml"
    }
}

#[cfg(test)]
mod test {
    use serde_derive::{Deserialize, Serialize};
    use toml::de::Error;

    use crate::model::metadata::MetaData;

    use super::super::deserializer::Deserializer;
    use super::TomlDeserializer;

    #[derive(Serialize, Deserialize, Debug, Clone)]
    #[serde(untagged)]
    pub enum TestStruct {
        /// The process is actually a `Flow`
        FlowProcess(String),
        /// The process is actually a `Function`
        FunctionProcess(String),
    }

    #[test]
    fn invalid_toml() {
        let toml = TomlDeserializer::<TestStruct>::new();
        if toml.deserialize("{}}}}f fake data ", None).is_ok() {
            panic!("Should not have parsed correctly as is invalid TOML");
        };
    }

    #[test]
    fn metadata() {
        let metadata = "\
name = \"me\"
version = \"1.1.1\"
description = \"ok\"
authors = [\"Andrew <andrew@foo.com>\"]
    ";

        let result: Result<MetaData, Error> = toml::from_str(metadata);
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

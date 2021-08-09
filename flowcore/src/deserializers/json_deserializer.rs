use std::marker::PhantomData;

use serde::Deserialize;
use url::Url;

use crate::errors::*;

use super::deserializer::Deserializer;

/// Struct representing a Generic deserializer of content stored in Json format
#[derive(Default)]
pub struct JsonDeserializer<'a, T>
where
    T: Deserialize<'a>,
{
    t: PhantomData<&'a T>,
}

impl<'a, T> JsonDeserializer<'a, T>
where
    T: Deserialize<'a>,
{
    /// Create a new JsonDeserializer
    pub fn new() -> Self {
        JsonDeserializer { t: PhantomData }
    }
}

impl<'a, T> Deserializer<'a, T> for JsonDeserializer<'a, T>
where
    T: Deserialize<'a>,
{
    fn deserialize(&self, contents: &'a str, url: Option<&Url>) -> Result<T> {
        serde_json::from_str(contents).chain_err(|| {
            format!(
                "Error deserializing Json from: '{}'",
                url.map_or("URL unknown".to_owned(), |u| u.to_string())
            )
        })
    }

    fn name(&self) -> &str {
        "Json"
    }
}

#[cfg(test)]
mod test {
    use serde_derive::{Deserialize, Serialize};

    use super::super::deserializer::Deserializer;
    use super::JsonDeserializer;

    #[derive(Serialize, Deserialize, Debug, Clone)]
    #[serde(untagged)]
    pub enum TestStruct {
        /// The process is actually a `Flow`
        FlowProcess(String),
        /// The process is actually a `Function`
        FunctionProcess(String),
    }

    #[test]
    fn invalid_json() {
        let json = JsonDeserializer::<TestStruct>::new();

        if json.deserialize("=", None).is_ok() {
            panic!("Should not have parsed correctly as is invalid JSON");
        };
    }
}

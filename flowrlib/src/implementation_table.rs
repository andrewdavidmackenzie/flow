use std::collections::HashMap;
use std::sync::Arc;

use flow_impl::implementation::Implementation;

use crate::errors::*;
use crate::provider::Provider;

/*
    Implementations can be of two types - either a native and statically bound function referenced
    via a function reference, or WASM bytecode file that is interpreted at run-time that is
    referenced via a PathBuf pointing to the .wasm file
*/
#[derive(Deserialize, Serialize)]
#[serde(untagged)]
pub enum ImplementationLocator {
    #[serde(skip_deserializing, skip_serializing)]
    Native(Arc<dyn Implementation>),
    Wasm((String, String))
}

const DEFAULT_ILT_FILENAME: &str = "ilt.json";

/*
    Provided by libraries to help load and/or find implementations of processes
*/
#[derive(Deserialize, Serialize)]
pub struct ImplementationLocatorTable {
    pub locators: HashMap<String, ImplementationLocator>
}

impl ImplementationLocatorTable {
    pub fn new() -> Self {
        ImplementationLocatorTable {
            locators: HashMap::<String, ImplementationLocator>::new()
        }
    }

    pub fn load(provider: &dyn Provider, source: &str) -> Result<ImplementationLocatorTable> {
        let (resolved_url, _) = provider.resolve(source, DEFAULT_ILT_FILENAME)?;
        let content = provider.get(&resolved_url)?;

        serde_json::from_str(
            &String::from_utf8(content).chain_err(|| "Could not convert from utf8 to String")?)
            .chain_err(|| format!("Could not read ILT as Json from '{}'", source))
    }
}

#[cfg(test)]
mod test {
    use crate::errors::*;
    use crate::implementation_table::ImplementationLocator;
    use crate::implementation_table::ImplementationLocator::Wasm;
    use crate::implementation_table::ImplementationLocatorTable;
    use crate::provider::Provider;

    pub struct TestProvider {
        test_content: &'static str
    }

    impl Provider for TestProvider {
        fn resolve(&self, source: &str, _default_filename: &str) -> Result<(String, Option<String>)> {
            Ok((source.to_string(), None))
        }

        fn get(&self, _url: &str) -> Result<Vec<u8>> {
            Ok(self.test_content.as_bytes().to_owned())
        }
    }

    #[test]
    fn serialize() {
        let locator: ImplementationLocator = Wasm(("add2.wasm".to_string(), "add".to_string()));
        let mut ilt = ImplementationLocatorTable::new();
        ilt.locators.insert("//flowrlib/test-dyn-lib/add2".to_string(), locator);
        let serialized = serde_json::to_string_pretty(&ilt).unwrap();
        let expected = "{
  \"locators\": {
    \"//flowrlib/test-dyn-lib/add2\": [
      \"add2.wasm\",
      \"add\"
    ]
  }
}";
        assert_eq!(expected, serialized);
    }

    #[test]
    fn load_dyn_library() {
        let test_content = "{
  \"locators\": {
    \"//flowrlib/test-dyn-lib/add2\": [\
        \"add2.wasm\",\
        \"add\"]
  }
}";
        let provider = TestProvider {
            test_content
        };
        let url = "file:://test/fake";
        let ilt = ImplementationLocatorTable::load(&provider, url).unwrap();
        assert_eq!(ilt.locators.len(), 1);
        assert!(ilt.locators.get("//flowrlib/test-dyn-lib/add2").is_some());
        let locator = ilt.locators.get("//flowrlib/test-dyn-lib/add2").unwrap();
        match locator {
            Wasm(source) => assert_eq!(source.0, "add2.wasm"),
            _ => assert!(false, "Expected type 'Wasm' but found another type")
        }
    }
}



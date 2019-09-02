use std::collections::HashMap;
use std::sync::Arc;

use flow_impl::implementation::Implementation;

use crate::errors::*;
use crate::manifest::MetaData;
use crate::provider::Provider;

/*
    Implementations can be of two types - either a native and statically bound function referenced
    via a function reference, or WASM bytecode file that is interpreted at run-time that is
    referenced via a string pointing to the .wasm file location
*/
#[derive(Deserialize, Serialize)]
#[serde(untagged)]
pub enum ImplementationLocator {
    #[serde(skip_deserializing, skip_serializing)]
    Native(Arc<dyn Implementation>),
    Wasm(String)
}

const DEFAULT_LIB_MANIFEST_FILENAME: &str = "manifest.json";

/*
    Provided by libraries to help load and/or find implementations of processes
*/
#[derive(Deserialize, Serialize)]
pub struct LibraryManifest {
    pub metadata: MetaData,
    pub locators: HashMap<String, ImplementationLocator>
}

impl LibraryManifest {
    pub fn new(metadata: MetaData) -> Self {
        LibraryManifest {
            metadata,
            locators: HashMap::<String, ImplementationLocator>::new()
        }
    }

    pub fn load(provider: &dyn Provider, source: &str) -> Result<(LibraryManifest, String)> {
        let (resolved_url, _) = provider.resolve(source, DEFAULT_LIB_MANIFEST_FILENAME)?;
        let content = provider.get(&resolved_url)
            .expect(&format!("Could not read contents of Library Manifest from '{}'", resolved_url));

        let manifest = serde_json::from_str(
            &String::from_utf8(content).chain_err(|| "Could not convert from utf8 to String")?)
            .chain_err(|| format!("Could not load Library Manfest from '{}'", resolved_url))?;

        Ok((manifest, resolved_url))
    }
}

#[cfg(test)]
mod test {
    use crate::errors::*;
    use crate::lib_manifest::{ImplementationLocator, ImplementationLocator::Wasm, LibraryManifest};
    use crate::manifest::MetaData;
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
        let metadata = MetaData {
            name: "".to_string(),
            description: "".into(),
            version: "0.1.0".into(),
            author_name: "".into(),
            author_email: "".into(),
        };

        let locator: ImplementationLocator = Wasm("add2.wasm".to_string());
        let mut manifest = LibraryManifest::new(metadata);
        manifest.locators.insert("//flowrlib/test-dyn-lib/add2".to_string(), locator);
        let serialized = serde_json::to_string_pretty(&manifest).unwrap();
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
        let (lib_manifest, _lib_manifest_url) = LibraryManifest::load(&provider, url).unwrap();
        assert_eq!(lib_manifest.locators.len(), 1);
        assert!(lib_manifest.locators.get("//flowrlib/test-dyn-lib/add2").is_some());
        let locator = lib_manifest.locators.get("//flowrlib/test-dyn-lib/add2").unwrap();
        match locator {
            Wasm(source) => assert_eq!(source, "add2.wasm"),
            _ => assert!(false, "Expected type 'Wasm' but found another type")
        }
    }
}



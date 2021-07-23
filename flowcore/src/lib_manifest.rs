use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use log::debug;
use serde_derive::{Deserialize, Serialize};
use url::Url;

use crate::errors::*;
use crate::flow_manifest::MetaData;
use crate::lib_provider::LibProvider;
use crate::Implementation;

/// The default name used for a Library  Manifest file if none is specified
pub const DEFAULT_LIB_JSON_MANIFEST_FILENAME: &str = "manifest";
/// The default name used for a Rust Library Manifest if none is specified
pub const DEFAULT_LIB_RUST_MANIFEST_FILENAME: &str = "lib.rs";

/*
    Implementations can be of two types - either a statically linked function referenced
    via a function reference, or WASM bytecode file that is interpreted at run-time that is
    referenced via a string pointing to the .wasm file location
*/
#[derive(Deserialize, Serialize, Clone)]
#[serde(untagged)]
/// Used to describe where an implementation can be found, depending on if native or wasm
pub enum ImplementationLocator {
    #[serde(skip_deserializing, skip_serializing)]
    /// A `Native` implementation is a reference to a trait object and linked with the library
    Native(Arc<dyn Implementation>),
    /// A `Wasm` implementation is compiled to wasm and loaded to a file at the path indicated by the `String`
    Wasm(String),
}

impl PartialEq for ImplementationLocator {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                ImplementationLocator::Wasm(self_source),
                ImplementationLocator::Wasm(other_source),
            ) => self_source == other_source,
            _ => false,
        }
    }
}

#[derive(Deserialize, Serialize, Clone)]
/// `LibraryManifest` describes the contents of a Library that can be referenced from a `flow`
/// It is provided by libraries to help load and/or find implementations of processes
pub struct LibraryManifest {
    /// `metadata` about a flow with author, version and usual fields
    pub metadata: MetaData,
    /// the `locators` map a reference to a function/implementation to the `ImplementationLocator`
    /// that can be used to load it or reference it
    pub locators: HashMap<Url, ImplementationLocator>,
    /// source_files is a list of source files (location relative to library root) for functions
    /// (function definitions and source code) and process flow definitions that form part of it
    #[serde(default)]
    pub source_urls: HashSet<(Url, Url)>,
}

impl LibraryManifest {
    /// Create a new, empty, `LibraryManifest` with the provided `Metadata`
    pub fn new(metadata: MetaData) -> Self {
        LibraryManifest {
            metadata,
            locators: HashMap::<Url, ImplementationLocator>::new(),
            source_urls: HashSet::<(Url, Url)>::new(),
        }
    }

    /// `load` a `LibraryManifest` from the `source` url, using the `provider` to fetch contents
    pub fn load(
        provider: &dyn LibProvider,
        lib_manifest_url: &Url,
    ) -> Result<(LibraryManifest, Url)> {
        let (resolved_url, _) = provider
            .resolve_url(
                lib_manifest_url,
                DEFAULT_LIB_JSON_MANIFEST_FILENAME,
                &["json"],
            )
            .chain_err(|| {
                format!(
                    "Could not resolve the library manifest url '{}'",
                    lib_manifest_url
                )
            })?;

        let manifest_content = provider.get_contents(&resolved_url).chain_err(|| {
            format!(
                "Could not read contents of Library Manifest from '{}'",
                resolved_url
            )
        })?;

        let manifest = serde_json::from_str(
            &String::from_utf8(manifest_content)
                .chain_err(|| "Could not deserialize LibraryManifest to JSON")?,
        )
        .chain_err(|| format!("Could not load LibraryManifest from '{}'", resolved_url))?;

        Ok((manifest, resolved_url))
    }

    /// Add a function's implementation locator (location of wasm file) to the library manifest
    /// It will be stored relative to the root of the manifest so that the library is location
    /// independent
    pub fn add_locator(
        &mut self,
        base_dir: &str,
        wasm_abs_path: &str,
        wasm_dir: &str,
        function_name: &str,
    ) -> Result<()> {
        let relative_dir = wasm_dir.replace(base_dir, "");
        let lib_reference = Url::parse(&format!(
            "lib://{}/{}/{}",
            self.metadata.name, relative_dir, function_name
        ))
        .chain_err(|| "Could not form library Url to add to the manifest")?;

        let implementation_relative_location = wasm_abs_path.replace(base_dir, "");
        debug!(
            "Adding implementation to manifest: \n'{}'  --> '{}'",
            lib_reference, implementation_relative_location
        );
        self.locators.insert(
            lib_reference,
            ImplementationLocator::Wasm(implementation_relative_location),
        );

        Ok(())
    }
}

impl PartialEq for LibraryManifest {
    fn eq(&self, other: &Self) -> bool {
        if self.metadata != other.metadata {
            return false;
        }

        if self.locators.len() != other.locators.len() {
            return false;
        }

        for locator in self.locators.iter() {
            // try and find locator with the same key in the other HashMap
            if let Some(other_impl_locator) = other.locators.get(locator.0) {
                if *other_impl_locator != *locator.1 {
                    return false;
                }
            } else {
                return false; // no such locator in the other HashMap
            }
        }

        true // if we made it here then everything is the same
    }
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use serde_json::Value;
    use url::Url;

    use crate::errors::Result;
    use crate::flow_manifest::MetaData;
    use crate::lib_manifest::{
        ImplementationLocator, ImplementationLocator::Wasm, LibraryManifest,
    };
    use crate::lib_provider::LibProvider;
    use crate::Implementation;

    pub struct TestProvider {
        test_content: &'static str,
    }

    fn test_meta_data() -> MetaData {
        MetaData {
            name: "test".into(),
            version: "0.0.0".into(),
            description: "a test".into(),
            authors: vec!["me".into()],
        }
    }

    fn test_meta_data2() -> MetaData {
        MetaData {
            name: "different".into(),
            version: "0.0.0".into(),
            description: "a test".into(),
            authors: vec!["me".to_string()],
        }
    }

    impl LibProvider for TestProvider {
        fn resolve_url(
            &self,
            source: &Url,
            _default_filename: &str,
            _extensions: &[&str],
        ) -> Result<(Url, Option<String>)> {
            Ok((source.clone(), None))
        }

        fn get_contents(&self, _url: &Url) -> Result<Vec<u8>> {
            Ok(self.test_content.as_bytes().to_owned())
        }
    }

    #[test]
    fn create() {
        let _ = LibraryManifest::new(test_meta_data());
    }

    #[test]
    fn wasm_locators_match() {
        let loc0 = ImplementationLocator::Wasm("location".into());
        let loc1 = ImplementationLocator::Wasm("location".into());

        assert!(loc0 == loc1);
    }

    #[test]
    fn wasm_locators_do_not_match() {
        let loc0 = ImplementationLocator::Wasm("location0".into());
        let loc1 = ImplementationLocator::Wasm("location1".into());

        assert!(loc0 != loc1);
    }

    #[test]
    fn locators_type_mismatch() {
        #[derive(Debug)]
        struct TestImpl {}

        impl Implementation for TestImpl {
            fn run(&self, _inputs: &[Value]) -> (Option<Value>, bool) {
                unimplemented!()
            }
        }
        let wasm_loc = ImplementationLocator::Wasm("wasm_location".into());
        let native_loc = ImplementationLocator::Native(Arc::new(TestImpl {}));

        assert!(wasm_loc != native_loc);
    }

    #[test]
    fn serialize() {
        let metadata = MetaData {
            name: "".to_string(),
            description: "".into(),
            version: "0.1.0".into(),
            authors: vec![],
        };

        let locator: ImplementationLocator = Wasm("add2.wasm".to_string());
        let mut manifest = LibraryManifest::new(metadata);
        manifest.locators.insert(
            Url::parse("lib://flowrlib/test-dyn-lib/add2").expect("Could not create Url"),
            locator,
        );
        let serialized =
            serde_json::to_string_pretty(&manifest).expect("Could not pretty print JSON");
        let expected = "{
  \"metadata\": {
    \"name\": \"\",
    \"version\": \"0.1.0\",
    \"description\": \"\",
    \"authors\": []
  },
  \"locators\": {
    \"lib://flowrlib/test-dyn-lib/add2\": \"add2.wasm\"
  },
  \"source_urls\": []
}";
        assert_eq!(expected, serialized);
    }

    #[test]
    fn load_dyn_library() {
        let test_content = "{
  \"metadata\": {
    \"name\": \"\",
    \"version\": \"0.1.0\",
    \"description\": \"\",
    \"authors\": []
  },
  \"locators\": {
    \"lib://flowrlib/test-dyn-lib/add2\": \"add2.wasm\"
  },
  \"source_urls\": []
}";
        let provider = &TestProvider { test_content } as &dyn LibProvider;
        let url = Url::parse("file:://test/fake").expect("Could not create Url");
        let (lib_manifest, _lib_manifest_url) =
            LibraryManifest::load(provider, &url).expect("Could not load manifest");
        assert_eq!(lib_manifest.locators.len(), 1);
        assert!(lib_manifest
            .locators
            .get(&Url::parse("lib://flowrlib/test-dyn-lib/add2").expect("Create Url error"))
            .is_some());
        let locator = lib_manifest
            .locators
            .get(&Url::parse("lib://flowrlib/test-dyn-lib/add2").expect("Create Url error"))
            .unwrap();
        match locator {
            Wasm(source) => assert_eq!(source, "add2.wasm"),
            _ => panic!("Expected type 'Wasm' but found another type"),
        }
    }

    #[test]
    fn add_to() {
        let mut library = LibraryManifest::new(test_meta_data());
        library
            .add_locator("/fake", "/bin/my.wasm", "/bin", "my function")
            .expect("Could not add to manifest");
        assert_eq!(
            library.locators.len(),
            1,
            "There should be one implementation location in the library manifest"
        );
    }

    #[test]
    fn compare_manifests_metadata_different() {
        let library1 = LibraryManifest::new(test_meta_data());
        let library2 = LibraryManifest::new(test_meta_data2());

        assert!(library1 != library2);
    }

    #[test]
    fn compare_manifests_num_locators_different() {
        let mut library1 = LibraryManifest::new(test_meta_data());
        library1
            .add_locator("/fake", "/bin/my.wasm", "/bin", "my function")
            .expect("Could not add to manifest");

        let library2 = LibraryManifest::new(test_meta_data());

        assert!(library1 != library2);
    }

    #[test]
    fn compare_manifests_locators_different() {
        let mut library1 = LibraryManifest::new(test_meta_data());
        library1
            .add_locator("/fake", "/bin/fake.wasm", "/bin", "my fake function")
            .expect("Could not add to manifest");

        let mut library2 = LibraryManifest::new(test_meta_data());
        library2
            .add_locator("/different", "/bin/my.wasm", "/bin", "my function")
            .expect("Could not add to manifest");

        assert!(library1 != library2);
    }

    #[test]
    fn compare_manifests_same() {
        let mut library1 = LibraryManifest::new(test_meta_data());
        library1
            .add_locator("/fake", "/bin/my.wasm", "/bin", "my function")
            .expect("Could not add to manifest");

        let mut library2 = LibraryManifest::new(test_meta_data());
        library2
            .add_locator("/fake", "/bin/my.wasm", "/bin", "my function")
            .expect("Could not add to manifest");

        assert!(library1 == library2);
    }
}

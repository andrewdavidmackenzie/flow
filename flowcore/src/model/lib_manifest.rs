use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use log::{debug, info};
use serde_derive::{Deserialize, Serialize};
use url::Url;

use crate::deserializers::deserializer::get_deserializer;
use crate::errors::*;
use crate::Implementation;
use crate::model::metadata::MetaData;
use crate::provider::Provider;

/// The default name used for a Library  Manifest file if none is specified
pub const DEFAULT_LIB_JSON_MANIFEST_FILENAME: &str = "manifest";
/// The default name used for a Rust Library Manifest if none is specified
pub const DEFAULT_LIB_RUST_MANIFEST_FILENAME: &str = "manifest.rs";

/*
    Implementations can be of two types - either a statically linked function referenced
    via a function reference, or WASM bytecode file that is interpreted at run-time that is
    referenced via a string pointing to the .wasm file location
*/
#[derive(Deserialize, Serialize, Clone)]
#[serde(untagged)]
/// `ImplementationLocator` describes where an implementation can be located.
pub enum ImplementationLocator {
    #[serde(skip_deserializing, skip_serializing)]
    /// A `Native` - A reference to a trait object statically linked with the library
    Native(Arc<dyn Implementation>),
    /// A `Wasm` - `String` indicating where the wasm file of implementation is located
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
    /// the Url that this library implements
    pub lib_url: Url,
    /// `metadata` about a flow with author, version and usual fields
    pub metadata: MetaData,
    /// the `locators` map a reference to a function/implementation to the `ImplementationLocator`
    /// that can be used to load it or reference it
    pub locators: BTreeMap<Url, ImplementationLocator>,
    /// source_files is a list of source files (location relative to library root) for functions
    /// (function definitions and source code) and process flow definitions that form part of it
    #[serde(default)]
    pub source_urls: BTreeSet<(Url, Url)>,
}

impl LibraryManifest {
    /// Create a new, empty, `LibraryManifest` with the provided `Metadata`
    pub fn new(lib_url: Url, metadata: MetaData) -> Self {
        LibraryManifest {
            lib_url,
            metadata,
            locators: BTreeMap::<Url, ImplementationLocator>::new(),
            source_urls: BTreeSet::<(Url, Url)>::new(),
        }
    }

    /// load a `LibraryManifest` from `lib_manifest_url`, using `provider` to fetch the contents
    pub fn load(provider: &dyn Provider, lib_manifest_url: &Url) -> Result<(LibraryManifest, Url)> {
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

        let url = resolved_url.clone();
        let content = String::from_utf8(manifest_content)
            .chain_err(|| "Could not convert from utf8 to String")?;
        let deserializer = get_deserializer::<LibraryManifest>(&resolved_url)?;
        let manifest = deserializer
            .deserialize(&content, Some(&resolved_url))
            .chain_err(|| format!("Could not create a LibraryManifest from '{resolved_url}'"))?;

        Ok((manifest, url))
    }

    /// Add an implementation locator (location of the wasm file) to the `LibraryManifest`
    /// The WASM file's path is relative to the library root so that the library is location independent
    pub fn add_locator(
        &mut self,
        wasm_relative_path: &str,
        relative_dir: &str,
    ) -> Result<()> {
        let lib_reference = Url::parse(&format!(
            "lib://{}/{relative_dir}",
            self.metadata.name
        ))
        .chain_err(|| "Could not form library Url to add to the manifest")?;

        debug!(
            "Adding implementation locator to lib manifest: \n'{}' -> '{}'",
            lib_reference, wasm_relative_path
        );
        self.locators.insert(
            lib_reference,
            ImplementationLocator::Wasm(wasm_relative_path.to_owned()),
        );

        Ok(())
    }

    /// Given an output directory, return a PathBuf to the json format manifest that should be
    /// generated inside it
    pub fn manifest_filename(base_dir: &Path) -> PathBuf {
        let mut filename = base_dir.to_path_buf();
        filename.push(DEFAULT_LIB_JSON_MANIFEST_FILENAME);
        filename.set_extension("json");
        filename
    }

    /// Generate a manifest for the library in JSON
    pub fn write_json(&self, json_manifest_filename: &Path) -> Result<()> {
        let mut manifest_file = File::create(json_manifest_filename)?;

        manifest_file.write_all(
            serde_json::to_string_pretty(self)
                .chain_err(|| "Could not pretty format the library manifest JSON contents")?
                .as_bytes(),
        )?;

        info!("Generated library JSON manifest at '{}'", json_manifest_filename.display());

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
    use crate::Implementation;
    use crate::model::lib_manifest::{
        ImplementationLocator, ImplementationLocator::Native, ImplementationLocator::Wasm, LibraryManifest,
    };
    use crate::model::metadata::MetaData;
    use crate::provider::Provider;

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

    impl Provider for TestProvider {
        fn resolve_url(
            &self,
            source: &Url,
            _default_filename: &str,
            _extensions: &[&str],
        ) -> Result<(Url, Option<Url>)> {
            Ok((source.clone(), None))
        }

        fn get_contents(&self, _url: &Url) -> Result<Vec<u8>> {
            Ok(self.test_content.as_bytes().to_owned())
        }
    }

    #[test]
    fn create() {
        let _ = LibraryManifest::new(
            Url::parse("lib://testlib").expect("Could not parse lib url"),
            test_meta_data(),
        );
    }

    #[test]
    fn wasm_locators_match() {
        let loc0 = Wasm("location".into());
        let loc1 = Wasm("location".into());

        assert!(loc0 == loc1);
    }

    #[test]
    fn wasm_locators_do_not_match() {
        let loc0 = Wasm("location0".into());
        let loc1 = Wasm("location1".into());

        assert!(loc0 != loc1);
    }

    #[test]
    fn locators_type_mismatch() {
        #[derive(Debug)]
        struct TestImpl {}

        impl Implementation for TestImpl {
            fn run(&self, _inputs: &[Value]) -> Result<(Option<Value>, bool)> {
                unimplemented!()
            }
        }
        let wasm_loc = Wasm("wasm_location".into());
        let native_loc = Native(Arc::new(TestImpl {}));

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
        let mut manifest = LibraryManifest::new(
            Url::parse("lib://testlib").expect("Could not parse lib url"),
            metadata,
        );
        manifest.locators.insert(
            Url::parse("lib://flowrlib/test-dyn-lib/add2").expect("Could not create Url"),
            locator,
        );
        let serialized =
            serde_json::to_string_pretty(&manifest).expect("Could not pretty print JSON");
        let expected = "{
  \"lib_url\": \"lib://testlib\",
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
  \"lib_url\": \"lib://flowrlib\",
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
        let test_provider = &TestProvider { test_content } as &dyn Provider;
        let url = Url::parse("file://test/fake.json").expect("Could not create Url");
        let (lib_manifest, _lib_manifest_url) =
            LibraryManifest::load(test_provider, &url).expect("Could not load manifest");
        assert_eq!(lib_manifest.locators.len(), 1);
        assert!(lib_manifest
            .locators
            .get(&Url::parse("lib://flowrlib/test-dyn-lib/add2").expect("Create Url error"))
            .is_some());
        let locator = lib_manifest
            .locators
            .get(&Url::parse("lib://flowrlib/test-dyn-lib/add2").expect("Create Url error"))
            .expect("Could not get locator for Url");
        match locator {
            Wasm(source) => assert_eq!(source, "add2.wasm"),
            _ => panic!("Expected type 'Wasm' but found another type"),
        }
    }

    #[test]
    fn add_to() {
        let mut library = LibraryManifest::new(
            Url::parse("lib://testlib").expect("Could not parse lib url"),
            test_meta_data(),
        );
        library
            .add_locator("/bin/my.wasm", "/bin")
            .expect("Could not add to manifest");
        assert_eq!(
            library.locators.len(),
            1,
            "There should be one implementation location in the library manifest"
        );
    }

    #[test]
    fn compare_manifests_metadata_different() {
        let library1 = LibraryManifest::new(
            Url::parse("lib://testlib1").expect("Could not parse lib url"),
            test_meta_data(),
        );
        let library2 = LibraryManifest::new(
            Url::parse("lib://testlib2").expect("Could not parse lib url"),
            test_meta_data2(),
        );

        assert!(library1 != library2);
    }

    #[test]
    fn compare_manifests_num_locators_different() {
        let mut library1 = LibraryManifest::new(
            Url::parse("lib://testlib1").expect("Could not parse lib url"),
            test_meta_data(),
        );
        library1
            .add_locator("/bin/my.wasm", "/bin")
            .expect("Could not add to manifest");

        let library2 = LibraryManifest::new(
            Url::parse("lib://testlib1").expect("Could not parse lib url"),
            test_meta_data(),
        );

        assert!(library1 != library2);
    }

    #[test]
    fn compare_manifests_locators_different() {
        let mut library1 = LibraryManifest::new(
            Url::parse("lib://testlib1").expect("Could not parse lib url"),
            test_meta_data(),
        );
        library1
            .add_locator("/bin/fake.wasm", "/bin")
            .expect("Could not add to manifest");

        let mut library2 = LibraryManifest::new(
            Url::parse("lib://testlib2").expect("Could not parse lib url"),
            test_meta_data(),
        );
        library2
            .add_locator("/bin/my.wasm", "/bin")
            .expect("Could not add to manifest");

        assert!(library1 != library2);
    }

    #[test]
    fn compare_manifests_same() {
        let mut library1 = LibraryManifest::new(
            Url::parse("lib://testlib1").expect("Could not parse lib url"),
            test_meta_data(),
        );
        library1
            .add_locator("/bin/my.wasm", "/bin")
            .expect("Could not add to manifest");

        let mut library2 = LibraryManifest::new(
            Url::parse("lib://testlib1").expect("Could not parse lib url"),
            test_meta_data(),
        );
        library2
            .add_locator("/bin/my.wasm", "/bin")
            .expect("Could not add to manifest");

        assert!(library1 == library2);
    }
}

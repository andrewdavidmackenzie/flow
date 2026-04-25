//! Library catalog management: loading manifests, adding functions, resolving sources.

use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

use log::{info, warn};
use url::Url;

use flowcore::model::lib_manifest::LibraryManifest;
use flowcore::model::process::Process;
use flowcore::provider::Provider;

use crate::file_ops;

/// Load full library catalogs and cache all definitions.
///
/// For each unique library root URL found in `lib_references`, loads the library
/// manifest and parses every function/flow definition in that library. For each
/// URL in `context_references`, parses the context function definition.
pub(crate) fn load_library_catalogs(
    lib_references: &BTreeSet<Url>,
) -> (HashMap<Url, LibraryManifest>, HashMap<Url, Process>) {
    let provider = file_ops::build_meta_provider();
    let arc_provider: Arc<dyn Provider> = Arc::new(provider);
    let mut library_cache = HashMap::new();
    let mut all_definitions = HashMap::new();

    // Extract unique library root URLs from lib_references
    // e.g., "lib://flowstdlib/math/add" -> "lib://flowstdlib"
    let mut lib_roots: BTreeSet<Url> = BTreeSet::new();
    for lib_ref in lib_references {
        if let Some(host) = lib_ref.host_str() {
            if let Ok(root_url) = Url::parse(&format!("lib://{host}")) {
                lib_roots.insert(root_url);
            }
        }
    }

    // Load each library's full manifest
    for lib_root in &lib_roots {
        match LibraryManifest::load(&arc_provider, lib_root) {
            Ok((manifest, _manifest_url)) => {
                info!(
                    "Loaded library manifest for '{}' with {} locators",
                    lib_root,
                    manifest.locators.len()
                );

                // Parse each function/flow in the manifest
                let meta_provider = file_ops::build_meta_provider();
                for locator_url in manifest.locators.keys() {
                    match flowrclib::compiler::parser::parse(locator_url, &meta_provider) {
                        Ok(process) => {
                            all_definitions.insert(locator_url.clone(), process);
                        }
                        Err(e) => {
                            warn!("Could not parse library definition '{locator_url}': {e}");
                        }
                    }
                }

                library_cache.insert(lib_root.clone(), manifest);
            }
            Err(e) => {
                warn!("Could not load library manifest for '{lib_root}': {e}");
            }
        }
    }

    // Discover and parse context functions from the flowrcli runner directory.
    // Only scan ~/.flow/runner/flowrcli/ since flowedit only supports the flowrcli runner,
    // and the MetaProvider is configured with that context root.
    let ctx_provider = file_ops::build_meta_provider();
    let runner_dir = std::env::var("HOME")
        .map(|h| {
            std::path::PathBuf::from(h)
                .join(".flow")
                .join("runner")
                .join("flowrcli")
        })
        .unwrap_or_default();
    if runner_dir.is_dir() {
        if let Ok(cats) = std::fs::read_dir(&runner_dir) {
            for cat_entry in cats.flatten() {
                let cat_path = cat_entry.path();
                if !cat_path.is_dir() {
                    continue;
                }
                let cat_name = cat_entry.file_name().to_string_lossy().to_string();
                if let Ok(funcs) = std::fs::read_dir(&cat_path) {
                    for func_entry in funcs.flatten() {
                        let func_path = func_entry.path();
                        if func_path.extension().and_then(|e| e.to_str()) == Some("toml") {
                            let func_name = func_path
                                .file_stem()
                                .map(|s| s.to_string_lossy().to_string())
                                .unwrap_or_default();
                            if !func_name.is_empty() {
                                let ctx_url_str = format!("context://{cat_name}/{func_name}");
                                if let Ok(ctx_url) = Url::parse(&ctx_url_str) {
                                    if let std::collections::hash_map::Entry::Vacant(entry) =
                                        all_definitions.entry(ctx_url)
                                    {
                                        match flowrclib::compiler::parser::parse(
                                            entry.key(),
                                            &ctx_provider,
                                        ) {
                                            Ok(process) => {
                                                entry.insert(process);
                                            }
                                            Err(e) => {
                                                warn!(
                                                    "Could not parse context function '{ctx_url_str}': {e}"
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    (library_cache, all_definitions)
}

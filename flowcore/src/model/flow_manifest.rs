#[cfg(feature = "debugger")]
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt;
use std::fmt::Display;

use serde_derive::{Deserialize, Serialize};
use url::Url;

use crate::deserializers::deserializer::deserialize;
use crate::errors::{Result, ResultExt};
use crate::model::flow_definition::FlowDefinition;
use crate::model::metadata::MetaData;
use crate::model::output_connection::OutputConnection;
use crate::model::runtime_function::RuntimeFunction;
use crate::provider::Provider;

/// The default name used for a flow Manifest file if none is specified
pub const DEFAULT_MANIFEST_FILENAME: &str = "manifest";

impl From<&FlowDefinition> for MetaData {
    fn from(flow: &FlowDefinition) -> Self {
        flow.metadata.clone()
    }
}

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
/// `Cargo` meta-data that can be used as a source of meta-data
pub struct Cargo {
    /// We are only interested in the `package` part - as a source of meta-data
    pub package: MetaData,
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq, Debug)]
/// Describes a flow's direct children: which sub-flow IDs it contains
pub struct FlowInfo {
    /// The unique process ID of this flow
    pub process_id: usize,
    /// The ID of the parent flow, if any
    pub parent_id: Option<usize>,
    /// IDs of direct child sub-flows
    pub sub_flow_ids: Vec<usize>,
    #[cfg(feature = "debugger")]
    #[serde(default, skip_serializing_if = "String::is_empty")]
    /// The name of this flow (for debugging display)
    pub name: String,
    #[cfg(feature = "debugger")]
    #[serde(default, skip_serializing_if = "String::is_empty")]
    /// The route of this flow (for debugging display)
    pub route: String,
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq, Debug)]
/// A `flows` `Manifest` describes it and describes all the `Functions` it uses as well as
/// a list of references to libraries.
pub struct FlowManifest {
    /// The `MetaData` about this flow
    metadata: MetaData,
    /// A list of the `lib_references` used by this flow
    lib_references: BTreeSet<Url>,
    /// A list of the `context_references` used by this flow
    context_references: BTreeSet<Url>,
    /// A list of `RuntimeFunctions` in this flow
    functions: HashMap<usize, RuntimeFunction>,
    /// Flow hierarchy: which sub-flows each flow contains
    #[serde(default)]
    flows: HashMap<usize, FlowInfo>,
    #[cfg(feature = "debugger")]
    /// A list of the source files used to build this `flow`
    source_urls: BTreeMap<String, Url>,
}

impl Display for FlowManifest {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if !self.flows.is_empty() {
            writeln!(f, "Flows:")?;
            for (id, flow) in &self.flows {
                let parent = flow
                    .parent_id
                    .map_or("root".to_string(), |p| format!("Flow #{p}"));
                writeln!(f, "  Flow #{id} (parent: {parent})")?;
                if !flow.sub_flow_ids.is_empty() {
                    writeln!(f, "    Sub-flows: {:?}", flow.sub_flow_ids)?;
                }
            }
        }
        writeln!(f, "Functions:")?;
        for function in self.functions.values() {
            writeln!(
                f,
                "  Function #{} Implementation: {}",
                function.id(),
                function.get_implementation_url()
            )?;
        }
        Ok(())
    }
}

impl FlowManifest {
    /// Create a new manifest that can then be added to, and used in serialization
    #[must_use]
    pub fn new(metadata: MetaData) -> Self {
        FlowManifest {
            metadata,
            lib_references: BTreeSet::<Url>::new(),
            context_references: BTreeSet::<Url>::new(),
            functions: HashMap::new(),
            flows: HashMap::new(),
            #[cfg(feature = "debugger")]
            source_urls: BTreeMap::<String, Url>::new(),
        }
    }

    /// Add a run-time Function to the manifest for use in serialization
    pub fn add_function(&mut self, function: RuntimeFunction) {
        self.functions.insert(function.id(), function);
    }

    /// Scan all output connections and mark destination inputs that receive
    /// internal connections. Must be called after all functions are added.
    pub fn mark_internal_inputs(&mut self) {
        let internal_targets: Vec<(usize, usize)> = self
            .functions
            .values()
            .flat_map(|f| {
                f.get_output_connections()
                    .iter()
                    .filter(|c| c.internal)
                    .map(|c| (c.destination_id, c.destination_io_number))
            })
            .collect();

        for (func_id, io_number) in internal_targets {
            if let Some(func) = self.functions.get_mut(&func_id) {
                func.set_input_receives_internal(io_number);
            }
        }
    }

    /// Add flow hierarchy info to the manifest
    pub fn add_flow_info(&mut self, flow_info: FlowInfo) {
        self.flows.insert(flow_info.process_id, flow_info);
    }

    /// Get the flow hierarchy
    #[must_use]
    pub fn flows(&self) -> &HashMap<usize, FlowInfo> {
        &self.flows
    }

    /// Get the list of functions in this manifest
    #[must_use]
    pub fn functions(&self) -> &HashMap<usize, RuntimeFunction> {
        &self.functions
    }

    /// Get the list of functions in this manifest
    pub fn get_functions(&mut self) -> &mut HashMap<usize, RuntimeFunction> {
        &mut self.functions
    }

    /// Take the map of functions out of this manifest
    #[must_use]
    pub fn take_functions(self) -> HashMap<usize, RuntimeFunction> {
        self.functions
    }

    /// Get the metadata structure for this manifest
    #[must_use]
    pub fn get_metadata(&self) -> &MetaData {
        &self.metadata
    }

    /// get the list of all library references in this manifest
    #[must_use]
    pub fn get_lib_references(&self) -> &BTreeSet<Url> {
        &self.lib_references
    }

    /// get the list of all context references in this manifest
    #[must_use]
    pub fn get_context_references(&self) -> &BTreeSet<Url> {
        &self.context_references
    }

    /// set the list of all library references in this manifest
    pub fn set_lib_references(&mut self, lib_references: &BTreeSet<Url>) {
        self.lib_references.clone_from(lib_references);
    }

    /// set the list of all context references in this manifest
    pub fn set_context_references(&mut self, context_references: &BTreeSet<Url>) {
        self.context_references.clone_from(context_references);
    }

    /// Add a new library reference (the name of a library) into the manifest
    pub fn add_lib_reference(&mut self, lib_reference: &Url) {
        self.lib_references.insert(lib_reference.clone());
    }

    /// Add a new context reference (the name of a library) into the manifest
    pub fn add_context_reference(&mut self, context_reference: &Url) {
        self.context_references.insert(context_reference.clone());
    }

    /// set the list of all source urls used in the flow
    #[cfg(feature = "debugger")]
    pub fn set_source_urls(&mut self, source_urls: BTreeMap<String, Url>) {
        self.source_urls = source_urls;
    }

    /// Get the list of source files used in the flow
    #[cfg(feature = "debugger")]
    #[must_use]
    pub fn get_source_urls(&self) -> &BTreeMap<String, Url> {
        &self.source_urls
    }

    /// Compute the external interface of a sub-flow: the connections that cross
    /// its boundary from/to the rest of the flow.
    ///
    /// Returns `(inputs, outputs)` where:
    /// - `inputs`: connections from functions outside the sub-flow to functions inside it
    /// - `outputs`: connections from functions inside the sub-flow to functions outside it
    ///
    /// Each entry is a clone of the `OutputConnection` from the source function.
    ///
    /// # Errors
    ///
    /// Returns an error if the target `flow_id` is not found in the manifest.
    pub fn subflow_interface(
        &self,
        flow_id: usize,
    ) -> Result<(Vec<OutputConnection>, Vec<OutputConnection>)> {
        if !self.flows.contains_key(&flow_id) {
            crate::bail!("Flow #{flow_id} not found in manifest");
        }

        // Collect all function IDs inside the sub-flow (recursively)
        let mut flow_ids = HashSet::new();
        Self::collect_descendant_flows(flow_id, &self.flows, &mut flow_ids);
        let inside: HashSet<usize> = self
            .functions
            .iter()
            .filter(|(_, f)| flow_ids.contains(&f.get_parent_id()))
            .map(|(&id, _)| id)
            .collect();

        let mut inputs = Vec::new();
        let mut outputs = Vec::new();

        for func in self.functions.values() {
            let source_inside = inside.contains(&func.id());
            for conn in func.get_output_connections() {
                let dest_inside = inside.contains(&conn.destination_id);
                if !source_inside && dest_inside {
                    // External -> Internal: sub-flow input
                    inputs.push(conn.clone());
                } else if source_inside && !dest_inside {
                    // Internal -> External: sub-flow output
                    outputs.push(conn.clone());
                }
            }
        }

        Ok((inputs, outputs))
    }

    /// Delegate a sub-flow: remove its internal functions and replace with a
    /// single proxy function that uses a `subflow://` implementation URL.
    ///
    /// The proxy function inherits the sub-flow's external input connections
    /// (as flow initializers) and the sub-flow's flow hierarchy entry is preserved
    /// but its `sub_flow_ids` are cleared.
    ///
    /// Returns `(extracted_manifest, input_map)` where `input_map` maps proxy
    /// input indices to `(destination_func_id, destination_io_number)` in the sub-flow.
    ///
    /// # Errors
    ///
    /// Returns an error if the `flow_id` is not found.
    pub fn delegate_subflow(
        &mut self,
        flow_id: usize,
    ) -> Result<(FlowManifest, Vec<(usize, usize)>)> {
        // Compute the interface BEFORE removing functions
        let (interface_inputs, _interface_outputs) = self.subflow_interface(flow_id)?;

        // Extract the sub-flow manifest
        let extracted = self.extract_subflow(flow_id)?;

        // Find all function IDs inside the sub-flow
        let mut flow_ids = HashSet::new();
        Self::collect_descendant_flows(flow_id, &self.flows, &mut flow_ids);
        let inside_func_ids: Vec<usize> = self
            .functions
            .iter()
            .filter(|(_, f)| flow_ids.contains(&f.get_parent_id()))
            .map(|(&id, _)| id)
            .collect();

        // Remove internal functions
        for func_id in &inside_func_ids {
            self.functions.remove(func_id);
        }

        // Remove descendant flow entries (but keep the target flow itself)
        for &fid in &flow_ids {
            if fid != flow_id {
                self.flows.remove(&fid);
            }
        }
        if let Some(flow_info) = self.flows.get_mut(&flow_id) {
            flow_info.sub_flow_ids.clear();
        }

        // Build proxy inputs — one for each unique interface input
        // Group by (destination_id, destination_io_number) to avoid duplicates
        let mut input_map: Vec<(usize, usize)> = interface_inputs
            .iter()
            .map(|c| (c.destination_id, c.destination_io_number))
            .collect();
        input_map.sort_unstable();
        input_map.dedup();

        let proxy_inputs: Vec<crate::model::input::Input> = input_map
            .iter()
            .enumerate()
            .map(|(input_idx, _)| {
                let _ = input_idx; // used only with debugger feature
                crate::model::input::Input::new(
                    #[cfg(feature = "debugger")]
                    format!("input_{input_idx}"),
                    0,
                    false,
                    None,
                    None,
                )
            })
            .collect();

        // Rewrite connections that targeted the delegated functions to
        // target the proxy function instead
        let inside_set: HashSet<usize> = inside_func_ids.iter().copied().collect();
        for func in self.functions.values_mut() {
            for conn in func.get_output_connections_mut() {
                if inside_set.contains(&conn.destination_id) {
                    // Find which proxy input this maps to
                    let key = (conn.destination_id, conn.destination_io_number);
                    if let Some(proxy_input_idx) = input_map.iter().position(|k| *k == key) {
                        conn.destination_id = flow_id;
                        conn.destination_io_number = proxy_input_idx;
                        conn.destination_parent_id = self
                            .flows
                            .get(&flow_id)
                            .and_then(|f| f.parent_id)
                            .unwrap_or(0);
                        conn.internal = false;
                    }
                }
            }
        }

        // Create proxy function
        let subflow_url = format!("subflow://{flow_id}");
        let parent_id = self
            .flows
            .get(&flow_id)
            .and_then(|f| f.parent_id)
            .unwrap_or(0);
        let mut proxy = crate::model::runtime_function::RuntimeFunction::new(
            #[cfg(feature = "debugger")]
            format!("subflow_{flow_id}"),
            #[cfg(feature = "debugger")]
            format!("/subflow/{flow_id}"),
            &subflow_url,
            proxy_inputs,
            flow_id,
            parent_id,
            &[], // output connections are in the boundary outputs
            false,
        );
        let dummy_base = Url::parse("file:///").map_err(|e| format!("{e}"))?;
        proxy.set_implementation_url(&dummy_base)?;
        self.functions.insert(flow_id, proxy);

        self.mark_internal_inputs();

        Ok((extracted, input_map))
    }

    /// Extract a sub-flow and all its descendants into a standalone `FlowManifest`.
    ///
    /// The target flow becomes the root of the new manifest (`parent_id` = `None`).
    /// All functions belonging to the target flow or any of its descendant sub-flows
    /// are included. Output connections that reference functions outside the extracted
    /// sub-flow are removed (they become the sub-flow's boundary).
    ///
    /// # Errors
    ///
    /// Returns an error if the target `flow_id` is not found in the manifest.
    pub fn extract_subflow(&self, flow_id: usize) -> Result<FlowManifest> {
        if !self.flows.contains_key(&flow_id) {
            crate::bail!("Flow #{flow_id} not found in manifest");
        }

        // Collect all descendant flow IDs (including the target itself)
        let mut flow_ids = HashSet::new();
        Self::collect_descendant_flows(flow_id, &self.flows, &mut flow_ids);

        // Collect all functions belonging to the target flow or its descendants
        let mut extracted_functions = HashMap::new();
        let function_ids: HashSet<usize> = self
            .functions
            .iter()
            .filter(|(_, f)| flow_ids.contains(&f.get_parent_id()))
            .map(|(&id, _)| id)
            .collect();

        for &func_id in &function_ids {
            if let Some(func) = self.functions.get(&func_id) {
                // Preserve all connections, including those targeting functions
                // outside the sub-flow (boundary outputs). The sub-flow executor
                // will intercept values sent to non-existent destinations and
                // relay them back to the parent coordinator.
                extracted_functions.insert(func_id, func.clone());
            }
        }

        // Build the flow hierarchy for the extracted sub-flow
        let mut extracted_flows = HashMap::new();
        for &fid in &flow_ids {
            if let Some(flow_info) = self.flows.get(&fid) {
                let mut cloned = flow_info.clone();
                // Make the target flow the root
                if fid == flow_id {
                    cloned.parent_id = None;
                }
                // Keep only sub-flow IDs that are in the extracted set
                cloned.sub_flow_ids.retain(|id| flow_ids.contains(id));
                extracted_flows.insert(fid, cloned);
            }
        }

        // Collect lib and context references from extracted functions.
        // Use implementation_location (the source string) rather than
        // implementation_url, which is only populated after manifest loading.
        let mut lib_refs = BTreeSet::new();
        let mut context_refs = BTreeSet::new();
        for func in extracted_functions.values() {
            let loc = func.get_implementation_location();
            if loc.starts_with("lib://") {
                if let Ok(url) = Url::parse(loc) {
                    lib_refs.insert(url);
                }
            } else if loc.starts_with("context://") {
                if let Ok(url) = Url::parse(loc) {
                    context_refs.insert(url);
                }
            }
        }

        #[cfg(feature = "debugger")]
        let flow_name = self
            .flows
            .get(&flow_id)
            .map_or_else(String::new, |f| f.name.clone());
        #[cfg(not(feature = "debugger"))]
        let flow_name = String::new();
        let metadata = MetaData {
            name: flow_name,
            ..MetaData::default()
        };

        let mut manifest = FlowManifest {
            metadata,
            lib_references: lib_refs,
            context_references: context_refs,
            functions: extracted_functions,
            flows: extracted_flows,
            #[cfg(feature = "debugger")]
            source_urls: BTreeMap::new(),
        };

        manifest.mark_internal_inputs();
        Ok(manifest)
    }

    /// Rewrite all `file://` implementation URLs in this manifest to `http://`
    /// URLs using the given WASM server base URL.
    ///
    /// This is used before sending a sub-flow manifest to a peer coordinator:
    /// the peer cannot access `file://` paths on the root machine, so the URLs
    /// are rewritten to point at the root's WASM HTTP server.
    ///
    /// Returns the number of URLs that were rewritten.
    pub fn rewrite_wasm_urls(&mut self, wasm_base_url: &Url) -> usize {
        let mut count = 0;
        for func in self.functions.values_mut() {
            if func.rewrite_file_url_to_http(wasm_base_url) {
                count += 1;
            }
        }
        count
    }

    /// Collect all descendant flow IDs (including the given flow itself).
    /// Uses an iterative work list to avoid stack overflow on deep hierarchies
    /// and skips already-visited IDs to handle cycles safely.
    fn collect_descendant_flows(
        flow_id: usize,
        flows: &HashMap<usize, FlowInfo>,
        result: &mut HashSet<usize>,
    ) {
        let mut work = vec![flow_id];
        while let Some(id) = work.pop() {
            if result.insert(id) {
                if let Some(flow_info) = flows.get(&id) {
                    work.extend(&flow_info.sub_flow_ids);
                }
            }
        }
    }

    /// Load, or Deserialize, a manifest from a `source` Url using `provider`
    /// Sets all `location_url` fields to be URLs, a file URL for provided implementations
    ///
    /// # Errors
    ///
    /// Returns `Err`if `manifest_url` cannot be resolved to a real url, the contents cannot be
    /// read from the resolved url, if the contents are not valid Utf8, or if the implementation
    /// url for the function definition is invalid
    pub fn load(provider: &dyn Provider, manifest_url: &Url) -> Result<(FlowManifest, Url)> {
        let (resolved_url, _) = provider
            .resolve_url(manifest_url, DEFAULT_MANIFEST_FILENAME, &["json"])
            .chain_err(|| "Could not resolve url for manifest.json")?;

        let contents = provider
            .get_contents(&resolved_url)
            .chain_err(|| "Could not get contents while attempting to load manifest")?;

        let url = resolved_url.clone();
        let content =
            String::from_utf8(contents).chain_err(|| "Could not convert from utf8 to String")?;
        let mut manifest: FlowManifest = deserialize(&resolved_url, &content)
            .chain_err(|| format!("Could not create a FlowManifest from '{manifest_url}'"))?;

        // normalize the implementation_locations into URLs.
        // context: and lib: URLs will be untouched
        // relative path locations to the manifest_url to file:// using the manifest Url as the base
        for function in manifest.functions.values_mut() {
            function.set_implementation_url(&resolved_url)?;
        }

        Ok((manifest, url))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {
    use url::Url;

    use crate::errors::Result;
    use crate::model::input::Input;
    use crate::model::runtime_function::RuntimeFunction;
    use crate::provider::Provider;

    use super::{FlowManifest, MetaData};

    fn test_meta_data() -> MetaData {
        MetaData {
            name: "test".into(),
            version: "0.0.0".into(),
            description: "a test".into(),
            authors: vec!["me".into()],
        }
    }

    #[allow(clippy::module_name_repetitions)]
    pub struct TestProvider {
        test_content: &'static str,
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
        let _ = FlowManifest::new(test_meta_data());
    }

    fn test_function() -> RuntimeFunction {
        RuntimeFunction::new(
            #[cfg(feature = "debugger")]
            "test",
            #[cfg(feature = "debugger")]
            "/test",
            "file://fake/test",
            vec![Input::new(
                #[cfg(feature = "debugger")]
                "",
                0,
                false,
                None,
                None,
            )],
            0,
            0,
            &[],
            false,
        )
    }

    #[test]
    fn add_function() {
        let function = test_function();

        let mut manifest = FlowManifest::new(test_meta_data());
        manifest.add_function(function);
        assert_eq!(manifest.functions.len(), 1);
    }

    #[test]
    fn load_manifest() {
        let test_content = "{
            \"metadata\": {
                \"name\": \"\",
                \"version\": \"0.1.0\",
                \"description\": \"\",
                \"authors\": []
                },
            \"manifest_dir\": \"fake dir\",
            \"lib_references\": [
             ],
            \"context_references\": [
                \"context://\"
             ],
            \"functions\": {
                \"0\": {
                    \"name\": \"print\",
                    \"route\": \"/print\",
                    \"process_id\": 0,
                    \"parent_id\": 0,
                    \"implementation_location\": \"context://stdio/stdout\",
                    \"inputs\": [ {} ]
                }
             },
            \"source_urls\": {}
            }";
        let provider = TestProvider { test_content };

        FlowManifest::load(
            &provider,
            &Url::parse("http://ibm.com/fake.json").expect("Could not parse URL"),
        )
        .expect("Could not load manifest");
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn extract_subflow_basic() {
        use super::FlowInfo;
        use crate::model::output_connection::{OutputConnection, Source};

        let mut manifest = FlowManifest::new(test_meta_data());

        // Flow hierarchy: root (#0) contains child (#1)
        // Root has function #10, child has functions #20 and #21
        manifest.add_flow_info(FlowInfo {
            process_id: 0,
            parent_id: None,
            sub_flow_ids: vec![1],
            #[cfg(feature = "debugger")]
            name: "root".into(),
            #[cfg(feature = "debugger")]
            route: "/root".into(),
        });
        manifest.add_flow_info(FlowInfo {
            process_id: 1,
            parent_id: Some(0),
            sub_flow_ids: vec![],
            #[cfg(feature = "debugger")]
            name: "child".into(),
            #[cfg(feature = "debugger")]
            route: "/root/child".into(),
        });

        // Function #10 in root, connects to #20 in child (cross-flow)
        manifest.add_function(RuntimeFunction::new(
            #[cfg(feature = "debugger")]
            "func10",
            #[cfg(feature = "debugger")]
            "/root/func10",
            "lib://flowstdlib/math/add",
            vec![Input::new(
                #[cfg(feature = "debugger")]
                "a",
                0,
                false,
                None,
                None,
            )],
            10,
            0,
            &[OutputConnection::new(
                Source::default(),
                20,
                0,
                1,
                false,
                String::new(),
                #[cfg(feature = "debugger")]
                String::new(),
            )],
            false,
        ));

        // Function #20 in child, connects to #21 (internal)
        manifest.add_function(RuntimeFunction::new(
            #[cfg(feature = "debugger")]
            "func20",
            #[cfg(feature = "debugger")]
            "/root/child/func20",
            "file://test/func20.wasm",
            vec![Input::new(
                #[cfg(feature = "debugger")]
                "in",
                0,
                false,
                None,
                None,
            )],
            20,
            1,
            &[OutputConnection::new(
                Source::default(),
                21,
                0,
                1,
                true,
                String::new(),
                #[cfg(feature = "debugger")]
                String::new(),
            )],
            false,
        ));

        // Function #21 in child, no outputs
        manifest.add_function(RuntimeFunction::new(
            #[cfg(feature = "debugger")]
            "func21",
            #[cfg(feature = "debugger")]
            "/root/child/func21",
            "file://test/func21.wasm",
            vec![Input::new(
                #[cfg(feature = "debugger")]
                "in",
                0,
                false,
                None,
                None,
            )],
            21,
            1,
            &[],
            false,
        ));

        manifest.mark_internal_inputs();

        // Extract child flow #1
        let extracted = manifest.extract_subflow(1).expect("extract failed");

        // Should contain functions #20 and #21 only
        assert_eq!(extracted.functions().len(), 2);
        assert!(extracted.functions().contains_key(&20));
        assert!(extracted.functions().contains_key(&21));
        assert!(!extracted.functions().contains_key(&10));

        // Child flow should be root (parent_id = None)
        assert_eq!(extracted.flows().len(), 1);
        assert!(extracted.flows().get(&1).unwrap().parent_id.is_none());

        // Internal connection #20 -> #21 should be preserved
        assert_eq!(
            extracted
                .functions()
                .get(&20)
                .unwrap()
                .get_output_connections()
                .len(),
            1
        );
    }

    #[test]
    fn extract_subflow_not_found() {
        let manifest = FlowManifest::new(test_meta_data());
        assert!(manifest.extract_subflow(99).is_err());
    }

    #[test]
    fn extract_subflow_handles_cycle() {
        use super::FlowInfo;

        let mut manifest = FlowManifest::new(test_meta_data());

        // Create a cycle: flow #0 -> flow #1 -> flow #0
        manifest.add_flow_info(FlowInfo {
            process_id: 0,
            parent_id: None,
            sub_flow_ids: vec![1],
            #[cfg(feature = "debugger")]
            name: "a".into(),
            #[cfg(feature = "debugger")]
            route: "/a".into(),
        });
        manifest.add_flow_info(FlowInfo {
            process_id: 1,
            parent_id: Some(0),
            sub_flow_ids: vec![0], // cycle back to root
            #[cfg(feature = "debugger")]
            name: "b".into(),
            #[cfg(feature = "debugger")]
            route: "/b".into(),
        });

        // Should not hang or panic — cycle is handled by visited set
        let result = manifest.extract_subflow(0);
        assert!(result.is_ok());
        let extracted = result.unwrap();
        assert_eq!(extracted.flows().len(), 2);
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn delegate_subflow_replaces_with_proxy() {
        use super::FlowInfo;
        use crate::model::output_connection::{OutputConnection, Source};

        let mut manifest = FlowManifest::new(test_meta_data());

        // Root flow with function #10, child flow with functions #20 and #21
        manifest.add_flow_info(FlowInfo {
            process_id: 0,
            parent_id: None,
            sub_flow_ids: vec![1],
            #[cfg(feature = "debugger")]
            name: "root".into(),
            #[cfg(feature = "debugger")]
            route: "/root".into(),
        });
        manifest.add_flow_info(FlowInfo {
            process_id: 1,
            parent_id: Some(0),
            sub_flow_ids: vec![],
            #[cfg(feature = "debugger")]
            name: "child".into(),
            #[cfg(feature = "debugger")]
            route: "/root/child".into(),
        });

        manifest.add_function(RuntimeFunction::new(
            #[cfg(feature = "debugger")]
            "func10",
            #[cfg(feature = "debugger")]
            "/root/func10",
            "lib://flowstdlib/math/add",
            vec![Input::new(
                #[cfg(feature = "debugger")]
                "a",
                0,
                false,
                None,
                None,
            )],
            10,
            0,
            // Connection from func10 (parent) into func20 (child flow)
            &[OutputConnection::new(
                Source::default(),
                20,
                0,
                1,
                false,
                String::new(),
                #[cfg(feature = "debugger")]
                String::new(),
            )],
            false,
        ));

        manifest.add_function(RuntimeFunction::new(
            #[cfg(feature = "debugger")]
            "func20",
            #[cfg(feature = "debugger")]
            "/root/child/func20",
            "file://test/func20.wasm",
            vec![Input::new(
                #[cfg(feature = "debugger")]
                "in",
                0,
                false,
                None,
                None,
            )],
            20,
            1,
            &[OutputConnection::new(
                Source::default(),
                21,
                0,
                1,
                true,
                String::new(),
                #[cfg(feature = "debugger")]
                String::new(),
            )],
            false,
        ));

        manifest.add_function(RuntimeFunction::new(
            #[cfg(feature = "debugger")]
            "func21",
            #[cfg(feature = "debugger")]
            "/root/child/func21",
            "file://test/func21.wasm",
            vec![Input::new(
                #[cfg(feature = "debugger")]
                "in",
                0,
                false,
                None,
                None,
            )],
            21,
            1,
            &[],
            false,
        ));

        // Delegate child flow #1
        let (extracted, input_map) = manifest.delegate_subflow(1).expect("delegate failed");

        // Extracted manifest should have the original child functions
        assert_eq!(extracted.functions().len(), 2);
        assert!(extracted.functions().contains_key(&20));
        assert!(extracted.functions().contains_key(&21));

        // Parent manifest should have func10 + proxy at flow_id 1
        assert_eq!(manifest.functions().len(), 2);
        assert!(manifest.functions().contains_key(&10));
        assert!(manifest.functions().contains_key(&1)); // proxy replaces flow

        // Proxy function should use subflow:// URL
        let proxy = manifest.functions().get(&1).unwrap();
        assert_eq!(proxy.get_implementation_location(), "subflow://1");
        assert_eq!(proxy.get_implementation_url().scheme(), "subflow");

        // input_map should map proxy input 0 to (func20, input 0)
        assert_eq!(input_map, vec![(20, 0)]);

        // Proxy should have one input (matching the one boundary connection)
        assert_eq!(proxy.inputs().len(), 1);

        // func10's connection should now target the proxy (id=1, io=0)
        // instead of func20 (id=20, io=0)
        let func10 = manifest.functions().get(&10).unwrap();
        let conns = func10.get_output_connections();
        assert_eq!(conns.len(), 1);
        let conn = conns.first().expect("should have one connection");
        assert_eq!(conn.destination_id, 1);
        assert_eq!(conn.destination_io_number, 0);
        assert!(!conn.internal);
    }

    #[test]
    fn rewrite_wasm_urls_rewrites_file_to_http() {
        use super::FlowInfo;

        let mut manifest = FlowManifest::new(test_meta_data());
        manifest.add_flow_info(FlowInfo {
            process_id: 0,
            parent_id: None,
            sub_flow_ids: vec![],
            #[cfg(feature = "debugger")]
            name: "root".into(),
            #[cfg(feature = "debugger")]
            route: "/root".into(),
        });

        // Add a file:// WASM function
        let mut func = RuntimeFunction::new(
            #[cfg(feature = "debugger")]
            "wasm_func",
            #[cfg(feature = "debugger")]
            "/root/wasm_func",
            "file:///path/to/module.wasm",
            vec![],
            1,
            0,
            &[],
            false,
        );
        let base = Url::parse("file:///").unwrap();
        func.set_implementation_url(&base).unwrap();
        manifest.add_function(func);

        // Add a lib:// function (should not be rewritten)
        let mut lib_func = RuntimeFunction::new(
            #[cfg(feature = "debugger")]
            "lib_func",
            #[cfg(feature = "debugger")]
            "/root/lib_func",
            "lib://flowstdlib/math/add",
            vec![],
            2,
            0,
            &[],
            false,
        );
        lib_func.set_implementation_url(&base).unwrap();
        manifest.add_function(lib_func);

        let wasm_base = Url::parse("http://192.168.1.1:12345").unwrap();
        let count = manifest.rewrite_wasm_urls(&wasm_base);

        assert_eq!(count, 1, "Should have rewritten exactly one URL");

        // The file:// function should now have an http:// URL
        let wasm_func = manifest.functions().get(&1).unwrap();
        assert_eq!(wasm_func.get_implementation_url().scheme(), "http");
        assert!(
            wasm_func
                .get_implementation_url()
                .as_str()
                .contains("module.wasm"),
            "Rewritten URL should contain the original path"
        );
        assert!(
            wasm_func
                .get_implementation_location()
                .starts_with("http://"),
            "implementation_location should also be rewritten"
        );

        // The lib:// function should be unchanged
        let lib_func = manifest.functions().get(&2).unwrap();
        assert_eq!(lib_func.get_implementation_url().scheme(), "lib");
    }

    #[test]
    fn rewrite_wasm_urls_no_file_urls_returns_zero() {
        let mut manifest = FlowManifest::new(test_meta_data());

        let mut lib_func = RuntimeFunction::new(
            #[cfg(feature = "debugger")]
            "lib_func",
            #[cfg(feature = "debugger")]
            "/root/lib_func",
            "lib://flowstdlib/math/add",
            vec![],
            1,
            0,
            &[],
            false,
        );
        let base = Url::parse("file:///").unwrap();
        lib_func.set_implementation_url(&base).unwrap();
        manifest.add_function(lib_func);

        let wasm_base = Url::parse("http://192.168.1.1:12345").unwrap();
        let count = manifest.rewrite_wasm_urls(&wasm_base);
        assert_eq!(count, 0, "No file:// URLs to rewrite");
    }
}

#![allow(missing_docs)]

//! Tests for sub-flow extraction and independent execution.

use std::path::PathBuf;
use std::process::{Command, Stdio};

use flowcore::model::flow_manifest::FlowManifest;
use flowcore::model::submission::Submission;
use flowcore::provider::Provider;
use flowrlib::run_state::RunState;

/// Test that a sub-flow can be extracted from a compiled manifest,
/// wrapped in a `Submission`, and used to construct a `RunState` that
/// initializes correctly.
#[test]
fn extract_and_init_subflow() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().expect("Could not find project root");

    let example_dir = project_root
        .join("flowr")
        .join("examples")
        .join("mandlebrot");

    // Compile the example if needed
    let compile_status = Command::new("flowc")
        .args(["-d", "-g", "-c", "-O", "-r", "flowrcli"])
        .arg(example_dir.to_str().expect("path"))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("Could not run flowc");
    assert!(compile_status.success(), "flowc compilation failed");

    // Load the manifest
    let manifest_path = example_dir.join("manifest.json");
    let manifest_url =
        url::Url::from_file_path(&manifest_path).expect("Could not create manifest URL");
    let provider = TestProvider;
    let (manifest, _) =
        FlowManifest::load(&provider, &manifest_url).expect("Could not load manifest");

    // Verify the manifest has the expected flow hierarchy
    assert!(
        !manifest.flows().is_empty(),
        "Manifest should have flow hierarchy"
    );

    // Find the "render" sub-flow (or any sub-flow)
    let root_flow = manifest
        .flows()
        .values()
        .find(|f| f.parent_id.is_none())
        .expect("No root flow found");

    assert!(
        !root_flow.sub_flow_ids.is_empty(),
        "Root flow should have sub-flows"
    );

    let subflow_id = *root_flow
        .sub_flow_ids
        .first()
        .expect("Root flow has no sub-flows");

    // Extract the sub-flow
    let extracted = manifest
        .extract_subflow(subflow_id)
        .expect("Could not extract sub-flow");

    // Verify the extraction produced a valid manifest
    assert!(
        !extracted.functions().is_empty(),
        "Extracted sub-flow should have functions"
    );
    assert!(
        !extracted.flows().is_empty(),
        "Extracted sub-flow should have flow hierarchy"
    );

    // The target flow should be root (parent_id = None)
    let extracted_root = extracted
        .flows()
        .values()
        .find(|f| f.parent_id.is_none())
        .expect("Extracted manifest should have a root flow");
    assert_eq!(
        extracted_root.process_id, subflow_id,
        "Root of extracted manifest should be the target sub-flow"
    );

    // All functions should belong to the extracted flow or its descendants
    let flow_ids: std::collections::HashSet<usize> = extracted.flows().keys().copied().collect();
    for func in extracted.functions().values() {
        assert!(
            flow_ids.contains(&func.get_parent_id()),
            "Function #{} has parent_id {} which is not in the extracted flows",
            func.id(),
            func.get_parent_id()
        );
    }

    // Create a Submission from the extracted manifest
    let submission = Submission::new(
        extracted,
        None,
        None,
        #[cfg(feature = "debugger")]
        false,
        #[cfg(feature = "trace")]
        None,
    );

    // Construct RunState — this validates the flow hierarchy is consistent
    let state = RunState::new(submission);

    // Verify the state is valid
    assert!(
        state.num_functions() > 0,
        "Sub-flow RunState should have functions"
    );
    assert!(
        state.num_processes() > 0,
        "Sub-flow RunState should have processes (functions + flows)"
    );

    println!(
        "Successfully extracted sub-flow #{subflow_id}: {} functions, {} processes",
        state.num_functions(),
        state.num_processes()
    );
}

/// Minimal provider that reads files from the filesystem.
struct TestProvider;

impl Provider for TestProvider {
    fn resolve_url(
        &self,
        url: &url::Url,
        _default_filename: &str,
        _extensions: &[&str],
    ) -> flowcore::errors::Result<(url::Url, Option<url::Url>)> {
        Ok((url.clone(), None))
    }

    fn get_contents(&self, url: &url::Url) -> flowcore::errors::Result<Vec<u8>> {
        let path = url
            .to_file_path()
            .map_err(|()| format!("Could not convert URL to path: {url}"))?;
        std::fs::read(&path).map_err(|e| format!("Could not read {}: {e}", path.display()).into())
    }
}

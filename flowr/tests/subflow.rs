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
#[allow(clippy::too_many_lines)]
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

    // Verify the extracted manifest contains ONLY the target flow and its
    // descendants — not sibling flows or extra roots from the source manifest
    let extracted_flow_ids: std::collections::HashSet<usize> =
        extracted.flows().keys().copied().collect();

    // Build the expected set: target flow + all descendants from the source
    let mut expected_flow_ids = std::collections::HashSet::new();
    let mut work = vec![subflow_id];
    while let Some(id) = work.pop() {
        if expected_flow_ids.insert(id) {
            if let Some(info) = manifest.flows().get(&id) {
                work.extend(&info.sub_flow_ids);
            }
        }
    }
    assert_eq!(
        extracted_flow_ids, expected_flow_ids,
        "Extracted flows should be exactly the target and its descendants"
    );

    // All functions should belong to the extracted flow hierarchy
    for func in extracted.functions().values() {
        assert!(
            extracted_flow_ids.contains(&func.get_parent_id()),
            "Function #{} has parent_id {} which is not in the extracted flows",
            func.id(),
            func.get_parent_id()
        );
    }

    // Functions from outside the sub-flow should NOT be present
    let extracted_func_ids: std::collections::HashSet<usize> =
        extracted.functions().keys().copied().collect();
    for (&func_id, func) in manifest.functions() {
        if !expected_flow_ids.contains(&func.get_parent_id()) {
            assert!(
                !extracted_func_ids.contains(&func_id),
                "Function #{func_id} from outside the sub-flow should not be extracted"
            );
        }
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

/// Test that `subflow_interface` correctly identifies the external connections
/// crossing a sub-flow's boundary.
#[test]
fn subflow_interface_identifies_boundary_connections() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().expect("Could not find project root");
    let example_dir = project_root
        .join("flowr")
        .join("examples")
        .join("mandlebrot");

    // Compile if needed
    let _ = Command::new("flowc")
        .args(["-d", "-g", "-c", "-O", "-r", "flowrcli"])
        .arg(example_dir.to_str().expect("path"))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    let manifest_path = example_dir.join("manifest.json");
    let manifest_url =
        url::Url::from_file_path(&manifest_path).expect("Could not create manifest URL");
    let provider = TestProvider;
    let (manifest, _) =
        FlowManifest::load(&provider, &manifest_url).expect("Could not load manifest");

    // Find the render sub-flow (flow #4)
    let render_flow_id = manifest
        .flows()
        .iter()
        .find(|(_, f)| {
            #[cfg(feature = "debugger")]
            {
                f.name == "render"
            }
            #[cfg(not(feature = "debugger"))]
            {
                // Find render by having parent_id = root and no sub-flows
                f.parent_id == Some(0) && f.sub_flow_ids.is_empty()
            }
        })
        .map(|(&id, _)| id)
        .expect("Could not find render sub-flow");

    let (inputs, outputs) = manifest
        .subflow_interface(render_flow_id)
        .expect("subflow_interface failed");

    // Render sub-flow should have external inputs (from get and enumerate)
    assert!(
        !inputs.is_empty(),
        "Render sub-flow should have external inputs"
    );

    // Render sub-flow is a sink (writes to image_buffer context function) —
    // all its output connections are internal, so no external outputs
    assert!(
        outputs.is_empty(),
        "Render sub-flow should have no external outputs (it's a sink), \
         but found {} outputs",
        outputs.len()
    );

    println!(
        "Render sub-flow #{render_flow_id} interface: {} inputs, {} outputs",
        inputs.len(),
        outputs.len()
    );
}

/// Test that a `SubFlowImplementation` can execute a simple sub-flow.
/// Creates a minimal flow with one flowstdlib function (add) that has
/// initializers, so it can run to completion without external inputs.
#[cfg_attr(target_os = "windows", ignore)]
#[test]
fn subflow_implementation_executes() {
    use flowcore::model::flow_manifest::FlowInfo;
    use flowcore::model::input::{Input, InputInitializer};
    use flowcore::model::metadata::MetaData;
    use flowcore::model::runtime_function::RuntimeFunction;
    use flowcore::Implementation;
    use flowrlib::subflow::SubFlowImplementation;
    use std::sync::Arc;

    // Build a minimal manifest: one flow with one function (add)
    // add has two inputs, both initialized with Once values
    let mut manifest = FlowManifest::new(MetaData::default());
    manifest.add_flow_info(FlowInfo {
        process_id: 0,
        parent_id: None,
        sub_flow_ids: vec![],
        #[cfg(feature = "debugger")]
        name: "test".into(),
        #[cfg(feature = "debugger")]
        route: "/test".into(),
    });

    let mut func = RuntimeFunction::new(
        #[cfg(feature = "debugger")]
        "add",
        #[cfg(feature = "debugger")]
        "/test/add",
        "lib://flowstdlib/math/add",
        vec![
            Input::new(
                #[cfg(feature = "debugger")]
                "i1",
                0,
                false,
                Some(InputInitializer::Once(serde_json::json!(2))),
                None,
            ),
            Input::new(
                #[cfg(feature = "debugger")]
                "i2",
                0,
                false,
                Some(InputInitializer::Once(serde_json::json!(3))),
                None,
            ),
        ],
        1, // process_id
        0, // parent_id
        &[],
        false,
    );
    // Set the implementation URL (normally done during manifest loading)
    let dummy_manifest_url =
        url::Url::parse("file:///dummy/manifest.json").expect("Could not parse URL");
    func.set_implementation_url(&dummy_manifest_url)
        .expect("Could not set implementation URL");
    manifest.add_function(func);

    let provider = Arc::new(TestProvider) as Arc<dyn Provider>;
    let implementation = SubFlowImplementation::new(manifest, provider);

    // Run it — the add function should compute 2 + 3 = 5
    let result = implementation.run(&[]);
    assert!(
        result.is_ok(),
        "SubFlowImplementation::run() failed: {:?}",
        result.err()
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

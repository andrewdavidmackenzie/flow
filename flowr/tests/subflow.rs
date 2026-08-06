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
#[cfg_attr(target_os = "windows", ignore)]
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
#[cfg_attr(target_os = "windows", ignore)]
#[test]
fn subflow_interface_identifies_boundary_connections() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().expect("Could not find project root");
    let example_dir = project_root
        .join("flowr")
        .join("examples")
        .join("mandlebrot");

    // Compile if needed
    let compile_status = Command::new("flowc")
        .args(["-d", "-g", "-c", "-O", "-r", "flowrcli"])
        .arg(example_dir.to_str().expect("path"))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("Could not run flowc");
    assert!(compile_status.success(), "flowc compilation failed");

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

    // Render sub-flow should have 5 external inputs (from get and enumerate)
    assert_eq!(
        inputs.len(),
        5,
        "Render sub-flow should have 5 external inputs"
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
    // No interface inputs — the function has its own Once initializers
    let implementation = SubFlowImplementation::new(manifest, provider, vec![]);

    // Run it — the add function should compute 2 + 3 = 5
    let result = implementation.run(&[]);
    assert!(
        result.is_ok(),
        "SubFlowImplementation::run() failed: {:?}",
        result.err()
    );
}

/// Test that `SubFlowImplementation` can receive injected inputs through the interface.
#[cfg_attr(target_os = "windows", ignore)]
#[test]
fn subflow_implementation_with_injected_inputs() {
    use flowcore::model::flow_manifest::FlowInfo;
    use flowcore::model::input::Input;
    use flowcore::model::metadata::MetaData;
    use flowcore::model::runtime_function::RuntimeFunction;
    use flowcore::Implementation;
    use flowrlib::subflow::{InterfaceInput, SubFlowImplementation};
    use std::sync::Arc;

    // Build a minimal manifest: add function with NO initializers
    // Inputs will be injected via the interface
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
                None, // no initializer — will be injected
                None,
            ),
            Input::new(
                #[cfg(feature = "debugger")]
                "i2",
                0,
                false,
                None, // no initializer — will be injected
                None,
            ),
        ],
        1, // process_id
        0, // parent_id
        &[],
        false,
    );
    let dummy_url = url::Url::parse("file:///dummy/manifest.json").expect("URL");
    func.set_implementation_url(&dummy_url).expect("set URL");
    manifest.add_function(func);

    let provider = Arc::new(TestProvider) as Arc<dyn Provider>;
    let interface_inputs = vec![
        InterfaceInput {
            destination_id: 1,
            destination_io_number: 0,
        },
        InterfaceInput {
            destination_id: 1,
            destination_io_number: 1,
        },
    ];
    let implementation = SubFlowImplementation::new(manifest, provider, interface_inputs);

    // Inject inputs: 10 + 20 = 30
    let result = implementation.run(&[serde_json::json!(10), serde_json::json!(20)]);
    assert!(
        result.is_ok(),
        "SubFlowImplementation with injected inputs failed: {:?}",
        result.err()
    );
}

/// Test that boundary outputs are captured when a sub-flow function produces
/// output on a connection targeting a function outside the sub-flow.
#[cfg_attr(target_os = "windows", ignore)]
#[test]
#[allow(clippy::too_many_lines)]
fn subflow_captures_boundary_outputs() {
    use flowcore::model::flow_manifest::FlowInfo;
    use flowcore::model::input::{Input, InputInitializer};
    use flowcore::model::metadata::MetaData;
    use flowcore::model::output_connection::{OutputConnection, Source};
    use flowcore::model::runtime_function::RuntimeFunction;
    use flowcore::Implementation;
    use flowrlib::subflow::SubFlowImplementation;
    use std::sync::Arc;

    // Build a manifest with:
    //   Flow #0 (root) has function #10
    //   Flow #1 (child, inside root) has function #20
    //   Function #20 (add) has Once initializers and connects to #10 (boundary output)
    let mut full_manifest = FlowManifest::new(MetaData::default());
    full_manifest.add_flow_info(FlowInfo {
        process_id: 0,
        parent_id: None,
        sub_flow_ids: vec![1],
        #[cfg(feature = "debugger")]
        name: "root".into(),
        #[cfg(feature = "debugger")]
        route: "/root".into(),
    });
    full_manifest.add_flow_info(FlowInfo {
        process_id: 1,
        parent_id: Some(0),
        sub_flow_ids: vec![],
        #[cfg(feature = "debugger")]
        name: "child".into(),
        #[cfg(feature = "debugger")]
        route: "/root/child".into(),
    });

    // Function #10 in root (the boundary output destination)
    full_manifest.add_function(RuntimeFunction::new(
        #[cfg(feature = "debugger")]
        "target",
        #[cfg(feature = "debugger")]
        "/root/target",
        "lib://flowstdlib/math/add",
        vec![Input::new(
            #[cfg(feature = "debugger")]
            "in",
            0,
            false,
            None,
            None,
        )],
        10,
        0,
        &[],
        false,
    ));

    // Function #20 in child flow — has initializers and outputs to #10
    let mut func20 = RuntimeFunction::new(
        #[cfg(feature = "debugger")]
        "add",
        #[cfg(feature = "debugger")]
        "/root/child/add",
        "lib://flowstdlib/math/add",
        vec![
            Input::new(
                #[cfg(feature = "debugger")]
                "i1",
                0,
                false,
                Some(InputInitializer::Once(serde_json::json!(7))),
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
        20,
        1, // parent = child flow
        &[OutputConnection::new(
            Source::default(),
            10,    // destination = function #10 in root (OUTSIDE sub-flow)
            0,     // destination_io_number
            0,     // destination_parent_id = root
            false, // not internal (crosses flow boundary)
            String::new(),
            #[cfg(feature = "debugger")]
            String::new(),
        )],
        false,
    );
    let dummy_url = url::Url::parse("file:///dummy/manifest.json").expect("URL");
    func20.set_implementation_url(&dummy_url).expect("set URL");
    full_manifest.add_function(func20);

    // Extract child flow #1 — this preserves the boundary output connection
    let extracted = full_manifest
        .extract_subflow(1)
        .expect("extract_subflow failed");

    // Verify the boundary output connection is preserved
    let func20_extracted = extracted.functions().get(&20).expect("func 20");
    assert_eq!(
        func20_extracted.get_output_connections().len(),
        1,
        "Boundary output connection should be preserved"
    );

    // Run the extracted sub-flow
    let provider = Arc::new(TestProvider) as Arc<dyn Provider>;
    let implementation = SubFlowImplementation::new(extracted, provider, vec![]);

    let result = implementation.run(&[]).expect("run failed");

    // The output should contain boundary outputs (7 + 3 = 10 sent to #10:0)
    let (output, _run_again) = result;
    let outputs = output.expect("Sub-flow should produce boundary outputs");
    let arr = outputs.as_array().expect("outputs should be array");
    assert!(!arr.is_empty(), "Should have at least one boundary output");

    // Check the first output targets function #10, input 0, value = 10
    let first = arr.first().expect("should have first element");
    assert_eq!(
        first
            .get("destination_id")
            .and_then(serde_json::Value::as_u64),
        Some(10),
        "Output should target function #10"
    );
    assert_eq!(
        first
            .get("destination_io_number")
            .and_then(serde_json::Value::as_u64),
        Some(0),
    );
    assert_eq!(
        first.get("value").and_then(serde_json::Value::as_i64),
        Some(10), // 7 + 3 = 10
        "Output value should be 10 (7 + 3)"
    );
}

/// Test that `Executor::add_subflow` registers a manifest that can be used
/// for `subflow://` URL resolution.
#[test]
fn executor_registers_subflow_manifest() {
    use flowcore::model::flow_manifest::FlowInfo;
    use flowcore::model::metadata::MetaData;
    use flowrlib::executor::Executor;

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

    let mut executor = Executor::new();
    let subflow_url = url::Url::parse("subflow://0").expect("subflow URL");
    executor
        .add_subflow(subflow_url, manifest)
        .expect("add_subflow should succeed");
}

/// End-to-end test: submit a sub-flow to a peer coordinator running in
/// a background thread, verify boundary outputs come back.
#[cfg_attr(target_os = "windows", ignore)]
#[test]
#[allow(clippy::too_many_lines)]
fn peer_coordinator_executes_subflow() {
    use flowcore::model::flow_manifest::FlowInfo;
    use flowcore::model::input::{Input, InputInitializer};
    use flowcore::model::metadata::MetaData;
    use flowcore::model::output_connection::{OutputConnection, Source};
    use flowcore::model::runtime_function::RuntimeFunction;
    use flowrlib::peer_client::PeerClient;
    use std::sync::Arc;

    // Build a sub-flow: add(7,3)=10 with boundary output to #10:0
    let mut manifest = FlowManifest::new(MetaData::default());
    manifest.add_flow_info(FlowInfo {
        process_id: 0,
        parent_id: None,
        sub_flow_ids: vec![],
        #[cfg(feature = "debugger")]
        name: "subflow".into(),
        #[cfg(feature = "debugger")]
        route: "/subflow".into(),
    });

    let mut func = RuntimeFunction::new(
        #[cfg(feature = "debugger")]
        "add",
        #[cfg(feature = "debugger")]
        "/subflow/add",
        "lib://flowstdlib/math/add",
        vec![
            Input::new(
                #[cfg(feature = "debugger")]
                "i1",
                0,
                false,
                Some(InputInitializer::Once(serde_json::json!(7))),
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
        1,
        0,
        &[OutputConnection::new(
            Source::default(),
            10,
            0,
            0,
            false,
            String::new(),
            #[cfg(feature = "debugger")]
            String::new(),
        )],
        false,
    );
    let dummy_url = url::Url::parse("file:///dummy/manifest.json").expect("URL");
    func.set_implementation_url(&dummy_url).expect("set URL");
    manifest.add_function(func);

    // Start a peer coordinator in a background thread
    let peer_port = portpicker::pick_unused_port().expect("port");
    let bind_address = format!("tcp://*:{peer_port}");
    let peer_thread = std::thread::spawn(move || {
        let zmq_context = zmq::Context::new();
        let mut handler = flowrlib::peer_submission_handler::PeerSubmissionHandler::new(
            &zmq_context,
            &bind_address,
        )
        .expect("handler");

        // Set up coordinator infrastructure
        let ports = (
            portpicker::pick_unused_port().expect("port"),
            portpicker::pick_unused_port().expect("port"),
            portpicker::pick_unused_port().expect("port"),
            portpicker::pick_unused_port().expect("port"),
        );
        let bind_addrs = (
            format!("tcp://*:{}", ports.0),
            format!("tcp://*:{}", ports.1),
            format!("tcp://*:{}", ports.2),
            format!("tcp://*:{}", ports.3),
        );
        let connect_addrs = (
            format!("tcp://127.0.0.1:{}", ports.0),
            format!("tcp://127.0.0.1:{}", ports.1),
            format!("tcp://127.0.0.1:{}", ports.2),
            format!("tcp://127.0.0.1:{}", ports.3),
        );

        let dispatcher = flowrlib::dispatcher::Dispatcher::new(&bind_addrs).expect("dispatcher");
        let provider = Arc::new(TestProvider) as Arc<dyn flowcore::provider::Provider>;

        let mut executor = flowrlib::executor::Executor::new();
        #[cfg(feature = "flowstdlib")]
        executor
            .add_lib(
                flowstdlib::manifest::get().expect("flowstdlib"),
                url::Url::parse("memory://").expect("url"),
            )
            .expect("add_lib");
        executor.start(
            &provider,
            1,
            &connect_addrs.0,
            &connect_addrs.2,
            &connect_addrs.3,
        );

        std::thread::sleep(std::time::Duration::from_millis(100));

        #[cfg(feature = "debugger")]
        let mut debug_handler = flowrlib::subflow::NoOpDebugHandler;

        let mut coordinator = flowrlib::coordinator::Coordinator::new(
            dispatcher,
            #[cfg(feature = "submission")]
            &mut handler,
            #[cfg(feature = "debugger")]
            &mut debug_handler,
        );

        // Run one submission then exit
        let _ = coordinator.submission_loop(false);
        let _ = coordinator.send_done();
        executor.wait();
    });

    // Give peer coordinator time to start
    std::thread::sleep(std::time::Duration::from_millis(200));

    // Connect as parent and submit the sub-flow
    let zmq_context = zmq::Context::new();
    let peer_address = format!("127.0.0.1:{peer_port}");
    let client = PeerClient::connect(&zmq_context, &peer_address).expect("connect");

    let outputs = client
        .submit_subflow(manifest, vec![])
        .expect("submit failed");

    // Should have boundary output: add(7,3)=10 -> #10:0
    assert!(
        !outputs.is_empty(),
        "Should have boundary outputs from peer"
    );
    assert_eq!(
        outputs.first().map(|o| &o.value),
        Some(&serde_json::json!(10))
    );
    assert_eq!(
        outputs.first().map(|o| o.connection.destination_id),
        Some(10)
    );

    // Peer exits after one submission (loop_forever = false), no need to signal done.
    // Drop the client to close the socket.
    drop(client);

    // Wait for peer thread
    let _ = peer_thread.join();
}

/// End-to-end test with a real flowrex process as peer coordinator.
/// Starts flowrex, discovers its peer-coordinator service via mDNS,
/// submits a sub-flow, and verifies boundary outputs.
#[cfg_attr(target_os = "windows", ignore)]
#[test]
#[allow(clippy::too_many_lines)]
fn flowrex_peer_coordinator_end_to_end() {
    use flowcore::model::flow_manifest::FlowInfo;
    use flowcore::model::input::{Input, InputInitializer};
    use flowcore::model::metadata::MetaData;
    use flowcore::model::output_connection::{OutputConnection, Source};
    use flowcore::model::runtime_function::RuntimeFunction;
    use flowrlib::peer_client::PeerClient;
    use flowrlib::peer_discovery::discover_peer_coordinators;
    use std::process::{Command as ProcessCommand, Stdio};
    use std::time::Duration;

    // Start flowrex as a peer coordinator
    let mut flowrex = ProcessCommand::new("flowrex")
        .args(["--threads", "1", "-v", "info"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Could not spawn flowrex");

    // Wait for flowrex to start and advertise its peer-coordinator service
    std::thread::sleep(Duration::from_secs(5));

    // Discover the peer coordinator
    let peers = discover_peer_coordinators(Duration::from_secs(5), None).expect("discovery failed");

    if peers.is_empty() {
        // Clean up and skip — mDNS may not be working in this environment
        flowrex.kill().ok();
        flowrex.wait().ok();
        eprintln!("No peer coordinators discovered — skipping test");
        return;
    }

    let peer_address = peers.first().expect("no peers").clone();

    // Build a sub-flow: add(7,3)=10 with boundary output to #10:0
    let mut manifest = FlowManifest::new(MetaData::default());
    manifest.add_flow_info(FlowInfo {
        process_id: 0,
        parent_id: None,
        sub_flow_ids: vec![],
        #[cfg(feature = "debugger")]
        name: "subflow".into(),
        #[cfg(feature = "debugger")]
        route: "/subflow".into(),
    });

    let mut func = RuntimeFunction::new(
        #[cfg(feature = "debugger")]
        "add",
        #[cfg(feature = "debugger")]
        "/subflow/add",
        "lib://flowstdlib/math/add",
        vec![
            Input::new(
                #[cfg(feature = "debugger")]
                "i1",
                0,
                false,
                Some(InputInitializer::Once(serde_json::json!(7))),
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
        1,
        0,
        &[OutputConnection::new(
            Source::default(),
            10,
            0,
            0,
            false,
            String::new(),
            #[cfg(feature = "debugger")]
            String::new(),
        )],
        false,
    );
    let dummy_url = url::Url::parse("file:///dummy/manifest.json").expect("URL");
    func.set_implementation_url(&dummy_url).expect("set URL");
    manifest.add_function(func);

    // Connect to the peer and submit the sub-flow
    let zmq_context = zmq::Context::new();
    let client = PeerClient::connect(&zmq_context, &peer_address).expect("connect failed");

    let outputs = client
        .submit_subflow(manifest, vec![])
        .expect("submit failed");

    // Verify boundary output: add(7,3) = 10
    assert!(
        !outputs.is_empty(),
        "Should have boundary outputs from flowrex peer"
    );
    assert_eq!(
        outputs.first().map(|o| &o.value),
        Some(&serde_json::json!(10))
    );

    // Clean up
    drop(client);
    flowrex.kill().ok();
    flowrex.wait().ok();
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

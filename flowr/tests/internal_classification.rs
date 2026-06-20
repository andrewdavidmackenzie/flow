#![allow(missing_docs)]

use std::path::PathBuf;

use flowcore::content::file_provider::FileProvider;
use flowcore::model::flow_manifest::FlowManifest;
use url::Url;

fn load_example_manifest(example_name: &str) -> FlowManifest {
    let sample_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join(example_name);

    utilities::compile_example(&sample_dir, "flowrcli");

    let manifest_url =
        Url::from_file_path(sample_dir.join("manifest.json")).expect("Could not create URL");
    let provider = FileProvider;
    let (manifest, _) =
        FlowManifest::load(&provider, &manifest_url).expect("Could not load manifest");
    manifest
}

fn verify_basic_invariants(manifest: &FlowManifest, example_name: &str) {
    for function in manifest.functions().values() {
        for conn in function.get_output_connections() {
            let dst_fn = manifest
                .functions()
                .get(&conn.destination_id)
                .unwrap_or_else(|| {
                    panic!(
                        "{example_name}: destination fn#{} not found",
                        conn.destination_id
                    )
                });
            let src_parent = function.get_parent_id();
            let dst_parent = dst_fn.get_parent_id();

            // Self-loopbacks must always be internal
            if function.id() == conn.destination_id {
                assert!(
                    conn.internal,
                    "{example_name}: self-loopback fn#{} should be internal",
                    function.id()
                );
            }

            // Cross-flow connections must always be external
            if src_parent != dst_parent {
                assert!(
                    !conn.internal,
                    "{example_name}: cross-flow fn#{}(flow={}) -> fn#{}(flow={}) should be external",
                    function.id(),
                    src_parent,
                    conn.destination_id,
                    dst_parent
                );
            }
        }
    }
}

#[test]
fn factorial_all_internal() {
    let manifest = load_example_manifest("factorial");
    verify_basic_invariants(&manifest, "factorial");

    // Factorial has no sub-flows — all connections should be internal
    for function in manifest.functions().values() {
        for conn in function.get_output_connections() {
            assert!(
                conn.internal,
                "factorial: fn#{} -> fn#{}:io{} should be internal (single flow)",
                function.id(),
                conn.destination_id,
                conn.destination_io_number
            );
        }
    }
}

#[test]
fn sequence_of_sequences_classification() {
    let manifest = load_example_manifest("sequence-of-sequences");
    verify_basic_invariants(&manifest, "sequence-of-sequences");
}

#[test]
fn router_classification() {
    let manifest = load_example_manifest("router");
    verify_basic_invariants(&manifest, "router");

    // The restructured router routes forward_sum and cross_distance through the parent.
    // Verify those connections are external even though endpoints are in the same flow.
    let mut found_parent_routed = false;
    for function in manifest.functions().values() {
        for conn in function.get_output_connections() {
            let dst_fn = manifest
                .functions()
                .get(&conn.destination_id)
                .expect("destination not found");
            // Same flow but external means it was routed through parent
            if function.get_parent_id() == dst_fn.get_parent_id() && !conn.internal {
                found_parent_routed = true;
            }
        }
    }
    assert!(
        found_parent_routed,
        "router: should have at least one parent-routed (crossed boundary) connection"
    );
}

#[test]
fn prime_classification() {
    let manifest = load_example_manifest("prime");
    verify_basic_invariants(&manifest, "prime");

    // The composites sub-flow (flat state machine) should have all internal connections
    // within its flow, and external connections from the root flow
    let mut has_internal = false;
    let mut has_external = false;
    for function in manifest.functions().values() {
        for conn in function.get_output_connections() {
            if conn.internal {
                has_internal = true;
            } else {
                has_external = true;
            }
        }
    }
    assert!(has_internal, "prime: should have internal connections");
    assert!(has_external, "prime: should have external connections");
}

#[test]
fn weather_station_classification() {
    let manifest = load_example_manifest("weather-station");
    verify_basic_invariants(&manifest, "weather-station");
}

#[test]
fn fibonacci_classification() {
    let manifest = load_example_manifest("fibonacci");
    verify_basic_invariants(&manifest, "fibonacci");

    // Fibonacci has no sub-flows — all connections should be internal
    for function in manifest.functions().values() {
        for conn in function.get_output_connections() {
            assert!(
                conn.internal,
                "fibonacci: fn#{} -> fn#{}:io{} should be internal",
                function.id(),
                conn.destination_id,
                conn.destination_io_number
            );
        }
    }
}

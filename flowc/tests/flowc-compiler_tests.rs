#![allow(missing_docs)]

#[cfg(feature = "debugger")]
use std::collections::BTreeMap;

use serde_json::json;
use tempfile::tempdir;
#[cfg(feature = "debugger")]
use url::Url;

use flowcore::meta_provider::MetaProvider;
use flowcore::model::input::InputInitializer::Once;
use flowcore::model::name::HasName;
use flowcore::model::name::Name;
use flowcore::model::process::Process::{FlowProcess, FunctionProcess};
use flowcore::model::route::HasRoute;
use flowcore::model::route::Route;
use flowrclib::compiler::{compile, parser};

#[path = "helper.rs"]
mod helper;

/// flowrclib integration tests
///
/// These tests are in flowc and not flowrclib as we want to keep flowrclib free of anything that
/// prevents it from being compiled to wasm32 for use in a browser or other wasm environment.
///
/// These tests use pre-written flow definition (toml) files on the file system and use filesystem
/// io to read them, which cannot be compiled to wasm, as no such concept (as stdio) exists in a
/// generic wasm execution environment.
///
/// They could be written as code (not reading files) and hence in flowrclib, but that's quite
/// a lot of work to construct each flow in code, and so for now I've taken the easy route to write
/// some test flow toml files and just compile those. Plus that also stresses the deserialization
/// and parsing.
///
/// An interim solution could be to have the files in the code as Strings and parse from there.
#[test]
fn object_to_array_connection() {
    let meta_provider = MetaProvider::new(helper::set_lib_search_path_to_project(),
                                          helper::get_canonical_context_root(),
    );
    let path = helper::absolute_file_url_from_relative_path(
        "flowc/tests/test-flows/object_to_array_connection/root.toml",
    );
    let process = parser::parse(&path, &meta_provider)
        .expect("Could not parse test flow");
    if let FlowProcess(ref flow) = process {
        let output_dir = tempdir().expect("A temp dir").keep();

        #[cfg(feature = "debugger")]
            let mut source_urls = BTreeMap::<String, Url>::new();

        let _tables = compile::compile(flow, &output_dir, false, false, &mut source_urls)
            .expect("Could not compile flow");
    } else {
        panic!("Process loaded was not a flow");
    }
}

#[test]
fn context_with_io() {
    let meta_provider = MetaProvider::new(helper::set_lib_search_path_to_project(),
                                          helper::get_canonical_context_root(),
    );
    let path = helper::absolute_file_url_from_relative_path(
        "flowc/tests/test-flows/context_with_io/root.toml",
    );
    let process = parser::parse(&path, &meta_provider)
        .expect("Could not parse test flow");
    if let FlowProcess(ref flow) = process {
        let output_dir = tempdir().expect("A temp dir").keep();

        #[cfg(feature = "debugger")]
            let mut source_urls = BTreeMap::<String, Url>::new();

        if compile::compile(flow, &output_dir, false, false, &mut source_urls).is_ok() {
            // flow loaded, but has ios
            assert!(!flow.inputs().is_empty());
            assert!(!flow.outputs().is_empty());
        }
    } else {
        panic!("Process loaded was not a flow");
    }
}

#[test]
fn same_name_input_and_output() {
    let meta_provider = MetaProvider::new(helper::set_lib_search_path_to_project(),
                                          helper::get_canonical_context_root(),
    );
    let path = helper::absolute_file_url_from_relative_path(
        "flowc/tests/test-flows/same-name-parent/root.toml",
    );
    let process = parser::parse(&path, &meta_provider)
        .expect("Could not parse test flow");
    if let FlowProcess(ref flow) = process {
        let output_dir = tempdir().expect("A temp dir").keep();

        #[cfg(feature = "debugger")]
            let mut source_urls = BTreeMap::<String, Url>::new();

        let tables = compile::compile(flow, &output_dir, false, false,
                                      &mut source_urls).expect("Could not compile flow");
        // If done correctly there should only be two connections
        assert_eq!(2, tables.collapsed_connections.len());
    } else {
        panic!("Process loaded was not a flow");
    }
}

#[test]
fn same_name_flow_ids() {
    let meta_provider = MetaProvider::new(helper::set_lib_search_path_to_project(),
                                          helper::get_canonical_context_root(),
    );
    let path = helper::absolute_file_url_from_relative_path(
        "flowc/tests/test-flows/same-name-parent/root.toml",
    );
    let process = parser::parse(&path, &meta_provider)
        .expect("Could not parse test flow");
    if let FlowProcess(ref flow) = process {
        let output_dir = tempdir().expect("A temp dir").keep();

        #[cfg(feature = "debugger")]
            let mut source_urls = BTreeMap::<String, Url>::new();

        let tables = compile::compile(flow, &output_dir, false, false,
                                      &mut source_urls).expect("Could not compile flow");

        // print function in context flow should have flow_id = 0
        let print_function = tables
            .functions
            .iter()
            .find(|f| f.alias() == &Name::from("stdout"))
            .expect("Could not find function named stdout");
        assert_eq!(
            print_function.get_flow_id(),
            0,
            "stdout function in context should have flow_id = 0"
        );
    } else {
        panic!("Process loaded was not a flow");
    }
}

#[test]
fn connection_to_input_with_constant_initializer() {
    let meta_provider = MetaProvider::new(helper::set_lib_search_path_to_project(),
                                          helper::get_canonical_context_root(),
    );
    let path = helper::absolute_file_url_from_relative_path(
        "flowc/tests/test-flows/connect_to_constant/root.toml",
    );
    let process = parser::parse(&path, &meta_provider)
        .expect("Could not parse test flow");
    if let FlowProcess(ref flow) = process {
        let output_dir = tempdir().expect("A temp dir").keep();

        #[cfg(feature = "debugger")]
            let mut source_urls = BTreeMap::<String, Url>::new();

        assert!(compile::compile(flow, &output_dir, false, false, &mut source_urls).is_err(),
                "Process should not have loaded due to connection to input with a constant initializer");
    } else {
        panic!("Process loaded was not a flow");
    }
}

#[test]
fn args() {
    let meta_provider = MetaProvider::new(helper::set_lib_search_path_to_project(),
                                          helper::get_canonical_context_root(),
    );
    let path =
        helper::absolute_file_url_from_relative_path("flowc/tests/test-flows/args/root.toml");
    let process = parser::parse(&path, &meta_provider)
        .expect("Could not parse test flow");
    if let FlowProcess(ref flow) = process {
        let output_dir = tempdir().expect("A temp dir").keep();

        #[cfg(feature = "debugger")]
            let mut source_urls = BTreeMap::<String, Url>::new();

        let _tables = compile::compile(flow, &output_dir, false, false,
                                       &mut source_urls).expect("Could not compile flow");
    } else {
        panic!("Process loaded was not a flow");
    }
}

#[test]
fn no_side_effects() {
    let meta_provider = MetaProvider::new(helper::set_lib_search_path_to_project(),
                                          helper::get_canonical_context_root(),
    );
    let path = helper::absolute_file_url_from_relative_path("flowc/tests/test-flows/no_side_effects/root.toml");
    let process = parser::parse(&path, &meta_provider)
        .expect("Could not parse test flow");
    match process {
        FlowProcess(ref flow) => {
            let output_dir = tempdir().expect("A temp dir").keep();

            #[cfg(feature = "debugger")]
                let mut source_urls = BTreeMap::<String, Url>::new();

            match compile::compile(flow, &output_dir, false, false,
                                   &mut source_urls) {
                Ok(_tables) => panic!("Flow should not compile when it has no side-effects"),
                Err(e) => assert_eq!("Flow has no side-effects", e.description()),
            }
        }
        FunctionProcess(_) => panic!("Did not load a FlowProcess as expected")
    }
}

#[test]
fn compile_echo_ok() {
    let meta_provider = MetaProvider::new(helper::set_lib_search_path_to_project(),
                                          helper::get_canonical_context_root(),
    );
    let process = parser::parse(
        &helper::absolute_file_url_from_relative_path("flowc/tests/test-flows/echo/root.toml"),
        &meta_provider).expect("Could not parse test flow");
    if let FlowProcess(ref flow) = process {
        let output_dir = tempdir().expect("A temp dir").keep();

        #[cfg(feature = "debugger")]
            let mut source_urls = BTreeMap::<String, Url>::new();

        let _tables = compile::compile(flow, &output_dir, false, false,
                                       &mut source_urls).expect("Could not compile flow");
    } else {
        panic!("Process loaded was not a flow");
    }
}

#[test]
fn compiler_detects_unused_input() {
    let meta_provider = MetaProvider::new(helper::set_lib_search_path_to_project(),
                                          helper::get_canonical_context_root(),
    );
    let process = parser::parse(
        &helper::absolute_file_url_from_relative_path(
            "flowc/tests/test-flows/unused_input/root.toml",
        ),
        &meta_provider).expect("Could not parse test flow");
    if let FlowProcess(ref flow) = process {
        let output_dir = tempdir().expect("A temp dir").keep();

        #[cfg(feature = "debugger")]
            let mut source_urls = BTreeMap::<String, Url>::new();

        assert!(
            compile::compile(flow, &output_dir, false, false,
                             &mut source_urls).is_err(),
            "Should not compile due to unused input"
        );
    } else {
        panic!("Process loaded was not a flow");
    }
}

/*
    This tests that an initializer on an input to a flow sub-process is passed into it and
    back out via a connection
*/
#[test]
fn flow_input_propagated_back_out() {
    let meta_provider = MetaProvider::new(helper::set_lib_search_path_to_project(),
                                          helper::get_canonical_context_root(),
    );
    // Relative path from project root to the test file
    let url = helper::absolute_file_url_from_relative_path(
        "flowc/tests/test-flows/flow_input_init/root.toml",
    );

    match parser::parse(&url, &meta_provider) {
        Ok(FlowProcess(context)) => {
            let output_dir = tempdir().expect("A temp dir").keep();

            #[cfg(feature = "debugger")]
                let mut source_urls = BTreeMap::<String, Url>::new();

            match compile::compile(&context, &output_dir, false, false,
                                   &mut source_urls) {
                Ok(_tables) => {}
                Err(error) => panic!("Couldn't compile the flow from test file at '{url}'\n{error}"),
            }
        }
        Ok(FunctionProcess(_)) => panic!("Unexpected compile result from test file at '{url}'"),
        Err(error) => panic!("Couldn't parse the flow from test file at '{url}'.\n{error}"),
    }
}

/*
    This tests that an initializer on an output of a sub-process is passed propagated out and set on destination
*/
#[test]
fn initialized_output_propagated() {
    let meta_provider = MetaProvider::new(helper::set_lib_search_path_to_project(),
                                          helper::get_canonical_context_root(),
    );
    // Relative path from project root to the test file
    let url = helper::absolute_file_url_from_relative_path(
        "flowc/tests/test-flows/print_subflow_output/root.toml",
    );

    match parser::parse(&url, &meta_provider) {
        Ok(FlowProcess(context)) => {
            let output_dir = tempdir().expect("A temp dir").keep();

            #[cfg(feature = "debugger")]
                let mut source_urls = BTreeMap::<String, Url>::new();

            match compile::compile(&context, &output_dir, false, false,
                                   &mut source_urls) {
                Ok(tables) => {
                    match tables
                        .functions
                        .iter()
                        .find(|&f| f.route() == &Route::from("/print_subflow_output/stdout"))
                    {
                        Some(print_function) => {
                            let in_input = print_function.get_inputs().first()
                                .expect("Could not get inputs");
                            match in_input.get_flow_initializer() {
                                Some(Once(one_time)) => assert_eq!(one_time, &json!("Hello")), // PASS
                                _ => panic!("Expected a InputInitializer to be present")
                            }
                        }
                        None => {
                            panic!("Could not find function at route '/print_subflow_output/stdout'")
                        }
                    }
                }
                Err(error) => panic!(
                    "Couldn't compile the flow from test file at '{url}'\n{error}"),
            }
        }
        Ok(FunctionProcess(_)) => panic!("Unexpected compile result from test file at '{url}'"),
        Err(error) => panic!("Couldn't parse the flow from test file at '{url}'.\n{error}"),
    }
}

/*
    This tests that an initializer on an input to a subflow (and a subsequent subflow) is propagated
    along to the eventual function that uses it.
*/
#[test]
fn initialized_input_to_subflow() {
    let meta_provider = MetaProvider::new(helper::set_lib_search_path_to_project(),
                                          helper::get_canonical_context_root(),
    );
    // Relative path from project root to the test file
    let url = helper::absolute_file_url_from_relative_path(
        "flowc/tests/test-flows/initialized_input_to_subflow/root.toml",
    );

    match parser::parse(&url, &meta_provider) {
        Ok(FlowProcess(root)) => {
            let output_dir = tempdir().expect("A temp dir").keep();

            #[cfg(feature = "debugger")]
                let mut source_urls = BTreeMap::<String, Url>::new();

            match compile::compile(&root, &output_dir, false, false,
                                   &mut source_urls) {
                Ok(tables) => {
                    match tables
                        .functions
                        .iter()
                        .find(|&f| f.route() == &Route::from("/initialized_input_to_subflow/subflow/subsubflow/stdout"))
                    {
                        Some(print_function) => {
                            let in_input = print_function.get_inputs().first()
                                .expect("Could not get inputs");

                            match in_input.get_flow_initializer() {
                                Some(Once(one_time)) => assert_eq!(one_time, &json!("Hello")), // PASS
                                _ => panic!("Expected a InputInitializer to be present")
                            }
                        }
                        None => {
                            panic!("Could not find function at route '/initialized_input_to_subflow/subflow/print'")
                        }
                    }
                }
                Err(error) => panic!(
                    "Couldn't compile the flow from test file at '{url}'\n{error}"),
            }
        }
        Ok(FunctionProcess(_)) => panic!("Unexpected compile result from test file at '{url}'"),
        Err(error) => panic!("Couldn't parse the flow from test file at '{url}'.\n{error}"),
    }
}

#[test]
fn json_indexing() {
    let meta_provider = MetaProvider::new(
        helper::set_lib_search_path_to_project(),
        helper::get_canonical_context_root(),
    );
    let path =
        helper::absolute_file_url_from_relative_path("flowc/tests/test-flows/json-indexing/root.toml");
    let process = parser::parse(&path, &meta_provider)
        .expect("Could not parse flow");
    if let FlowProcess(ref flow) = process {
        let output_dir = tempdir().expect("A temp dir").keep();

        #[cfg(feature = "debugger")]
            let mut source_urls = BTreeMap::<String, Url>::new();

        let _tables = compile::compile(flow, &output_dir, false, false,
                                       &mut source_urls).expect("Could not compile flow");
    } else {
        panic!("Process loaded was not a flow");
    }
}
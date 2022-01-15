use std::collections::HashSet;

use serde_json::json;
use url::Url;

use flowclib::compiler::compile;
use flowclib::compiler::loader;
use flowclib::model::name::HasName;
use flowclib::model::name::Name;
use flowclib::model::process::Process::{FlowProcess, FunctionProcess};
use flowclib::model::route::HasRoute;
use flowclib::model::route::Route;
use flowcore::input::InputInitializer::Once;
use flowcore::lib_provider::MetaProvider;

#[path = "helper.rs"]
mod helper;

/// flowclib integration tests
///
/// These tests are in flowc and not flowclib as we want to keep flowclib free of anything that
/// prevents it from being compiled to wasm32 for use in a browser or other wasm environment.
///
/// These tests use pre-written flow definition (toml) files on the file system and use filesystem
/// io to read them, which cannot be compiled to wasm, as no such concept (as stdio) exists in a
/// generic wasm execution environment.
///
/// They could be written as code (not reading files) and hence in flowclib, but that's quite
/// a lot of work to construct each flow in code, and so for now I've taken the easy route to write
/// some test flow toml files and just compile those. Plus that also stresses the deserialization
/// and parsing.
///
/// An interim solution could be to have the files in the code as Strings and parse from there.
#[test]
fn args() {
    let meta_provider = MetaProvider::new(helper::set_lib_search_path_to_project());
    let path =
        helper::absolute_file_url_from_relative_path("flowc/tests/test-flows/args/args.toml");
    let process = loader::load(&path, &meta_provider, &mut HashSet::<(Url, Url)>::new()).unwrap();
    if let FlowProcess(ref flow) = process {
        let _tables = compile::compile(flow).unwrap();
    } else {
        panic!("Process loaded was not a flow");
    }
}

#[test]
fn object_to_array_connection() {
    let meta_provider = MetaProvider::new(helper::set_lib_search_path_to_project());
    let path = helper::absolute_file_url_from_relative_path(
        "flowc/tests/test-flows/object_to_array_connection/object_to_array_connection.toml",
    );
    let process = loader::load(&path, &meta_provider, &mut HashSet::<(Url, Url)>::new()).unwrap();
    if let FlowProcess(ref flow) = process {
        let _tables = compile::compile(flow).unwrap();
    } else {
        panic!("Process loaded was not a flow");
    }
}

#[test]
fn context_with_io() {
    let meta_provider = MetaProvider::new(helper::set_lib_search_path_to_project());
    let path = helper::absolute_file_url_from_relative_path(
        "flowc/tests/test-flows/context_with_io/context_with_io.toml",
    );
    let process = loader::load(&path, &meta_provider, &mut HashSet::<(Url, Url)>::new()).unwrap();
    if let FlowProcess(ref flow) = process {
        if compile::compile(flow).is_ok() {
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
    let meta_provider = MetaProvider::new(helper::set_lib_search_path_to_project());
    let path = helper::absolute_file_url_from_relative_path(
        "flowc/tests/test-flows/same-name-parent/same-name-parent.toml",
    );
    let process = loader::load(&path, &meta_provider, &mut HashSet::<(Url, Url)>::new()).unwrap();
    if let FlowProcess(ref flow) = process {
        let tables = compile::compile(flow).unwrap();
        // If done correctly there should only be two connections
        assert_eq!(2, tables.collapsed_connections.len());
    } else {
        panic!("Process loaded was not a flow");
    }
}

#[test]
fn same_name_flow_ids() {
    let meta_provider = MetaProvider::new(helper::set_lib_search_path_to_project());
    let path = helper::absolute_file_url_from_relative_path(
        "flowc/tests/test-flows/same-name-parent/same-name-parent.toml",
    );
    let process = loader::load(&path, &meta_provider, &mut HashSet::<(Url, Url)>::new()).unwrap();
    if let FlowProcess(ref flow) = process {
        let tables = compile::compile(flow).unwrap();

        // print function in context flow should have flow_id = 0
        let print_function = tables
            .functions
            .iter()
            .find(|f| f.alias() == &Name::from("print"))
            .unwrap();
        assert_eq!(
            print_function.get_flow_id(),
            0,
            "print function in context should have flow_id = 0"
        );
    } else {
        panic!("Process loaded was not a flow");
    }
}

#[test]
fn connection_to_input_with_constant_initializer() {
    let meta_provider = MetaProvider::new(helper::set_lib_search_path_to_project());
    let path = helper::absolute_file_url_from_relative_path(
        "flowc/tests/test-flows/connect_to_constant/connect_to_constant.toml",
    );
    let process = loader::load(&path, &meta_provider, &mut HashSet::<(Url, Url)>::new()).unwrap();
    if let FlowProcess(ref flow) = process {
        if compile::compile(flow).is_ok() {
            panic!("Process should not have loaded due to connection to input with a constant initializer");
        }
    } else {
        panic!("Process loaded was not a flow");
    }
}

#[test]
fn no_side_effects() {
    let meta_provider = MetaProvider::new(helper::set_lib_search_path_to_project());
    let path = helper::absolute_file_url_from_relative_path("flowc/tests/test-flows/no_side_effects/no_side_effects.toml");
    match loader::load(&path, &meta_provider, &mut HashSet::<(Url, Url)>::new()) {
        Ok(process) => {
            match process {
                FlowProcess(ref flow) => {
                    match compile::compile(flow) {
                        Ok(_tables) => panic!("Flow should not compile when it has no side-effects"),
                        Err(e) => assert_eq!("Flow has no side-effects", e.description()),
                    }
                }
                _ => panic!("Did not load a FlowProcess as expected")
            }
        }
        Err(e) => panic!("Could not load the test flow as expected: {}", e)
    }
}

#[test]
fn compile_echo_ok() {
    let meta_provider = MetaProvider::new(helper::set_lib_search_path_to_project());
    let process = loader::load(
        &helper::absolute_file_url_from_relative_path("flowc/tests/test-flows/echo/echo.toml"),
        &meta_provider,
        &mut HashSet::<(Url, Url)>::new(),
    )
    .unwrap();
    if let FlowProcess(ref flow) = process {
        let _tables = compile::compile(flow).unwrap();
    } else {
        panic!("Process loaded was not a flow");
    }
}

#[test]
fn compiler_detects_unused_input() {
    let meta_provider = MetaProvider::new(helper::set_lib_search_path_to_project());
    let process = loader::load(
        &helper::absolute_file_url_from_relative_path(
            "flowc/tests/test-flows/unused_input/unused_input.toml",
        ),
        &meta_provider,
        &mut HashSet::<(Url, Url)>::new(),
    )
    .unwrap();
    if let FlowProcess(ref flow) = process {
        assert!(
            compile::compile(flow).is_err(),
            "Should not compile due to unused input"
        );
    } else {
        panic!("Process loaded was not a flow");
    }
}

#[test]
fn compile_detects_connection_to_initialized_input() {
    let meta_provider = MetaProvider::new(helper::set_lib_search_path_to_project());
    let process = loader::load(
        &helper::absolute_file_url_from_relative_path(
            "flowc/tests/test-flows/connect_to_constant/connect_to_constant.toml",
        ),
        &meta_provider,
        &mut HashSet::<(Url, Url)>::new(),
    )
    .unwrap();
    if let FlowProcess(ref flow) = process {
        assert!(
            compile::compile(flow).is_err(),
            "Should not compile due to connection to constant initialized input"
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
    let meta_provider = MetaProvider::new(helper::set_lib_search_path_to_project());
    // Relative path from project root to the test file
    let url = helper::absolute_file_url_from_relative_path(
        "flowc/tests/test-flows/flow_input_init/flow_input_init.toml",
    );

    match loader::load(&url, &meta_provider, &mut HashSet::<(Url, Url)>::new()) {
        Ok(FlowProcess(context)) => match compile::compile(&context) {
            Ok(_tables) => {}
            Err(error) => panic!(
                "Couldn't compile the flow from test file at '{}'\n{}",
                url, error
            ),
        },
        Ok(FunctionProcess(_)) => panic!("Unexpected compile result from test file at '{}'", url),
        Err(error) => panic!(
            "Couldn't load the flow from test file at '{}'.\n{}",
            url, error
        ),
    }
}

/*
    This tests that an initializer on an output of a sub-process is passed propagated out and set on destination
*/
#[test]
fn initialized_output_propagated() {
    let meta_provider = MetaProvider::new(helper::set_lib_search_path_to_project());
    // Relative path from project root to the test file
    let url = helper::absolute_file_url_from_relative_path(
        "flowc/tests/test-flows/print_subflow_output/print_subflow_output.toml",
    );

    match loader::load(&url, &meta_provider, &mut HashSet::<(Url, Url)>::new()) {
        Ok(FlowProcess(context)) => {
            match compile::compile(&context) {
                Ok(tables) => {
                    match tables
                        .functions
                        .iter()
                        .find(|&f| f.route() == &Route::from("/print_subflow_output/print"))
                    {
                        Some(print_function) => {
                            let in_input = print_function.get_inputs().get(0).unwrap();
                            let initial_value = in_input.get_initializer();
                            match initial_value {
                                Some(Once(one_time)) => assert_eq!(one_time, &json!("Hello")), // PASS
                                _ => panic!(
                                    "Initializer should have been a Once initializer, was {:?}",
                                    initial_value
                                ),
                            }
                        }
                        None => {
                            panic!("Could not find function at route '/print_subflow_output/print'")
                        }
                    }
                }
                Err(error) => panic!(
                    "Couldn't compile the flow from test file at '{}'\n{}",
                    url, error
                ),
            }
        }
        Ok(FunctionProcess(_)) => panic!("Unexpected compile result from test file at '{}'", url),
        Err(error) => panic!(
            "Couldn't load the flow from test file at '{}'.\n{}",
            url, error
        ),
    }
}
use flowclib::compiler::loader;
use flowclib::model::io::IO;
use flowclib::model::name::HasName;
use flowclib::model::name::Name;
use flowclib::model::process::Process::FlowProcess;
use flowclib::model::process::Process::FunctionProcess;
use flowclib::model::route::HasRoute;
use flowclib::model::route::Route;
use flowrstructs::input::InputInitializer::Once;
use provider::content::provider::MetaProvider;

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
/// An interim solution could be so have the files in the code as Strings and parse from there.
///
#[test]
fn malformed_connection() {
    let meta_provider = MetaProvider {};
    let path = helper::absolute_file_url_from_relative_path("flowc/tests/test-flows/malformed-connection.toml");
    if loader::load(&path, &meta_provider).is_ok() {
        panic!("malformed-connection.toml should not load successfully");
    }
}

#[test]
fn invalid_toml() {
    let meta_provider = MetaProvider {};
    let path = helper::absolute_file_url_from_relative_path("flowc/tests/test-flows/invalid.toml");
    if loader::load(&path, &meta_provider).is_ok() {
        panic!("invalid.toml should not load successfully");
    }
}

#[test]
fn invalid_process() {
    let meta_provider = MetaProvider {};
    let path = helper::absolute_file_url_from_relative_path("flowc/tests/test-flows/invalid-process/invalid-process.toml");
    if loader::load(&path, &meta_provider).is_ok() {
        panic!("invalid.toml should not load successfully");
    }
}

#[test]
fn function_input_initialized() {
    let meta_provider = MetaProvider {};
    let url = helper::absolute_file_url_from_relative_path("flowc/tests/test-flows/function_input_init/function_input_init.toml");

    match loader::load(&url, &meta_provider) {
        Ok(FlowProcess(mut flow)) => {
            match flow.subprocesses.get_mut(&Name::from("print")) {
                Some(FunctionProcess(print_function)) => {
                    assert_eq!(*print_function.alias(), Name::from("print"), "Function alias does not match");
                    if let Some(inputs) = print_function.get_inputs() {
                        let default_input: &IO = inputs.get(0).unwrap();
                        let initial_value = default_input.get_initializer().clone().unwrap();
                        match initial_value {
                            Once(one_time) => assert_eq!(one_time, "hello"),
                            _ => panic!("Initializer should have been a Once initializer")
                        }
                    } else {
                        panic!("Could not find any inputs");
                    }
                }
                _ => panic!("Sub-process was not a Function")
            }
        }
        Ok(_) => panic!("Didn't load a flow"),
        Err(e) => panic!(e.to_string())
    }
}

#[test]
fn root_flow_takes_name_from_file() {
    let meta_provider = MetaProvider {};
    // Relative path from project root to the test file
    let url = helper::absolute_file_url_from_relative_path("flowc/tests/test-flows/names/names.toml");

    match loader::load(&url, &meta_provider) {
        Ok(FlowProcess(flow)) => assert_eq!(flow.name, Name::from("names")),
        _ => panic!("Flow could not be loaded")
    }
}

/*
    This tests that an initializer on an input to a flow process is passed onto function processes
    inside the flow, via a connection from the flow input to the function input
*/
#[test]
fn flow_input_initialized_and_propagated_to_function() {
    let meta_provider = MetaProvider {};
    // Relative path from project root to the test file
    let url = helper::absolute_file_url_from_relative_path("flowc/tests/test-flows/flow_input_init/flow_input_init.toml");

    match loader::load(&url, &meta_provider) {
        Ok(FlowProcess(mut flow)) => {
            match flow.subprocesses.get_mut(&Name::from("count")) {
                Some(FlowProcess(sub_flow)) => {
                    assert_eq!(Name::from("count"), *sub_flow.alias(), "Flow alias is not 'count' as expected");
                    match sub_flow.subprocesses.get_mut(&Name::from("compare")) {
                        Some(FunctionProcess(ref tap_function)) => {
                            assert_eq!(Name::from("compare"), *tap_function.alias(), "Function alias is not 'compare' as expected");
                            if let Some(inputs) = tap_function.get_inputs() {
                                let in_input = inputs.get(0).unwrap();
                                assert_eq!(Name::from("left"), *in_input.alias(), "Input's name is not 'left' as expected");
                                assert_eq!(Route::from("/flow_input_init/count/compare/left"), *in_input.route(), "Input's route is not as expected");
                                let initial_value = in_input.get_initializer();
                                match initial_value {
                                    Some(Once(one_time)) => assert_eq!(one_time, 10),
                                    _ => panic!("Initializer should have been a Once initializer")
                                }
                            } else {
                                panic!("Could not find any inputs");
                            }
                        }
                        _ => panic!("The expected function sub-process of 'pass-if-lte' was not found")
                    }
                }
                _ => panic!("The expected function 'print' sub-process was not found")
            }
        },
        Ok(_) => panic!("Didn't find a Flow as expected"),
        Err(err) => panic!("Didn't load a flow process as expected. {}", err.to_string())
    }
}

#[test]
fn load_library() {
    let path = helper::absolute_file_url_from_relative_path("flowc/tests/test_libs/Cargo.toml");
    let provider = MetaProvider {};

    loader::load_metadata(&path, &provider).unwrap();
}
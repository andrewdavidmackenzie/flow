use std::env;

use flowclib::compiler::loader;
use flowclib::model::io::IO;
use flowclib::model::name::HasName;
use flowclib::model::name::Name;
use flowclib::model::process::Process::FlowProcess;
use flowclib::model::process::Process::FunctionProcess;
use flowclib::model::route::HasRoute;
use flowclib::model::route::Route;
use flowrlib::input::InputInitializer::OneTime;
use provider::content::provider::MetaProvider;
use url::Url;

/// flowclib integration tests
///
/// These tests are in flowc and not flowclib as we want to keep flowclib free of anything that
/// prevents it from being compiled to wasm32 for use in a browser or other wasm environment.
///
/// These tests use pre-written flow definition (toml) files on the file system and use filesystem
/// io to read them, which cannot be compiled to wasm, as no such concept (as stdio) exists in a
/// generic wasm execution environment.
///
/// They could be written as pure code (not reading files) and hence in flowclib, but that's quite
/// a lot of work to construct each flow in code, and so for now I've taken the easy route to write
/// some test flow toml files and just compile those. Plus that also stresses the deserialization
/// and parsing.
///
/// An interim solution could be so have the files in the code as Strings and parse from there.
///

fn set_flow_lib_path() {
    let mut parent_dir = std::env::current_dir().unwrap();
    parent_dir.pop();
    println!("Set 'FLOW_LIB_PATH' to '{}'", parent_dir.to_string_lossy().to_string());
    env::set_var("FLOW_LIB_PATH", parent_dir.to_string_lossy().to_string());
}

// Helper function for tests
fn url_from_rel_path(path: &str) -> String {
    let cwd = Url::from_file_path(env::current_dir().unwrap()).unwrap();
    let source_file = cwd.join(file!()).unwrap();
    let file = source_file.join(path).unwrap();
    file.to_string()
}

#[test]
fn malformed_connection() {
    set_flow_lib_path();
    let meta_provider = MetaProvider {};
    let path = url_from_rel_path("test-flows/malformed-connection.toml");
    let result = loader::load_context(&path, &meta_provider);
    match result {
        Ok(_) => assert!(false, "malformed-connection.toml should not load successfully"),
        Err(_) => { /* error was correctly detected but didn't cause a crash */ }
    }
}

#[test]
fn invalid_toml() {
    set_flow_lib_path();
    let meta_provider = MetaProvider {};
    let path = url_from_rel_path("test-flows/invalid.toml");
    let result = loader::load_context(&path, &meta_provider);
    match result {
        Ok(_) => assert!(false, "invalid.toml should not load successfully"),
        Err(_) => { /* error was correctly detected but didn't cause a crash */ }
    }
}

#[test]
fn invalid_process() {
    set_flow_lib_path();
    let meta_provider = MetaProvider {};
    let path = url_from_rel_path("test-flows/invalid-process.toml");
    let result = loader::load_context(&path, &meta_provider);
    match result {
        Ok(_) => assert!(false, "invalid.toml should not load successfully"),
        Err(_) => { /* error was correctly detected but didn't cause a crash */ }
    }
}

#[test]
fn function_input_initialized() {
    set_flow_lib_path();
    let meta_provider = MetaProvider {};
    let url = url_from_rel_path("test-flows/function_input_init.toml");

    match loader::load_context(&url, &meta_provider) {
        Ok(FlowProcess(flow)) => {
            if let FunctionProcess(ref print_function) = flow.process_refs.unwrap()[0].process {
                assert_eq!(*print_function.alias(), Name::from("print"), "Function alias does not match");
                if let Some(inputs) = print_function.get_inputs() {
                    let default_input: &IO = inputs.get(0).unwrap();
                    let initial_value = default_input.get_initializer().clone().unwrap();
                    match initial_value {
                        OneTime(one_time) => assert_eq!(one_time.once, "hello"),
                        _ => panic!("Initializer should have been a OneTime initializer")
                    }
                } else {
                    panic!("Could not find any inputs");
                }
            } else {
                panic!("First sub-process was not a function as expected")
            }
        }
        Ok(_) => panic!("Didn't load a flow"),
        Err(e) => panic!(e.to_string())
    }
}

/*
    This tests that an initalizer on an input to a flow process is passed onto function processes
    inside the flow, via a connection from the flow input to the function input
*/
#[test]
fn flow_input_initialized_and_propogated_to_function() {
    set_flow_lib_path();
    let meta_provider = MetaProvider {};
    // Relative path from project root to the test file
    let url = url_from_rel_path("test-flows/flow_input_init.toml");

    match loader::load_context(&url, &meta_provider) {
        Ok(FlowProcess(flow)) => {
            if let FlowProcess(ref pilte_sub_flow) = flow.process_refs.unwrap()[0].process {
                assert_eq!(Name::from("pass-if-lte"), *pilte_sub_flow.alias(), "Flow alias is not 'pass-if-lte' as expected");

                if let Some(ref process_refs) = pilte_sub_flow.process_refs {
                    if let FunctionProcess(ref tap_function) = process_refs.get(0).unwrap().process {
                        assert_eq!(Name::from("tap"), *tap_function.alias(), "Function alias is not 'tap' as expected");
                        if let Some(inputs) = tap_function.get_inputs() {
                            let in_input = inputs.get(0).unwrap();
                            assert_eq!(Name::from("data"), *in_input.alias(), "Input's name is not 'data' as expected");
                            assert_eq!(Route::from("/context/pass-if-lte/tap/data"), *in_input.route(), "Input's route is not as expected");
                            let initial_value = in_input.get_initializer();
                            match initial_value {
                                Some(OneTime(one_time)) => assert_eq!(one_time.once, 1),
                                _ => panic!("Initializer should have been a OneTime initializer")
                            }
                        } else {
                            panic!("Could not find any inputs");
                        }
                    } else {
                        panic!("First sub-process of 'pass-if-lte' sub-flow was not a function as expected");
                    }
                } else {
                    panic!("Could not get process_refs of sub_flow");
                }
            } else {
                panic!("First sub-process of context flow was not a sub-flow as expected")
            }
        }
        Ok(_) => panic!("Didn't load a flow"),
        Err(e) => panic!(e.to_string())
    }
}


/*
    This tests that an initalizer on an input to a flow process is passed onto a function in
    a sub-flow of that via a connection from the flow input to the function input
*/
#[test]
#[should_panic]
fn flow_input_initialized_and_propogated_to_function_in_subflow() {
    set_flow_lib_path();
    let meta_provider = MetaProvider {};
    // Relative path from project root to the test file
    let url = url_from_rel_path("test-flows/subflow_function_input_init.toml");

    match loader::load_context(&url, &meta_provider) {
        Ok(FlowProcess(context)) => {
            if let FlowProcess(ref sequence_sub_flow) = context.process_refs.unwrap()[0].process {
                assert_eq!(Name::from("sequence"), *sequence_sub_flow.alias(), "First sub-flow alias is not 'sequence' as expected");

                if let Some(ref sequence_process_refs) = sequence_sub_flow.process_refs {
                    if let FlowProcess(ref pilte_sub_flow) = sequence_process_refs.get(0).unwrap().process {
                        assert_eq!(Name::from("pilte"), *pilte_sub_flow.alias(), "Sub-flow alias is not 'pilte' as expected");

                        if let Some(ref process_refs) = pilte_sub_flow.process_refs {
                            if let FunctionProcess(ref tap_function) = process_refs.get(0).unwrap().process {
                                assert_eq!(Name::from("tap"), *tap_function.alias(), "Function alias is not 'tap' as expected");
                                if let Some(inputs) = tap_function.get_inputs() {
                                    let in_input = inputs.get(0).unwrap();
                                    assert_eq!(Name::from("data"), *in_input.alias(), "Input's name is not 'data' as expected");
                                    assert_eq!(Route::from("/context/sequence/pilte/tap/data"), *in_input.route(), "Input's route is not as expected");
                                    let initial_value = in_input.get_initializer();
                                    match initial_value {
                                        Some(OneTime(one_time)) => assert_eq!(one_time.once, 1),
                                        _ => panic!("Initializer should have been a OneTime initializer")
                                    }
                                } else {
                                    panic!("Could not find any inputs");
                                }
                            } else {
                                panic!("First sub-process of 'pass-if-lte' sub-flow was not a function as expected");
                            }
                        } else {
                            panic!("Could not get process_refs of pilte_sub_flow");
                        }
                    } else {
                        panic!("Second sub-process of sequence sub-flow was not another sub-flow as expected");
                    }
                } else {
                    panic!("Could not get process_refs of sequence sub_flow");
                }
            } else {
                panic!("First sub-process of flow was not a sub-flow as expected");
            }
        }
        Ok(_) => panic!("Didn't load a flow"),
        Err(e) => panic!(e.to_string())
    }
}

#[test]
fn load_library() {
    let path = url_from_rel_path("test_libs/Library_test.toml");
    let provider = MetaProvider {};

    loader::load_library(&path, &provider).unwrap();
}
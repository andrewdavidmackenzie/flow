use flowclib::compiler::compile;
use flowclib::compiler::loader;
use flowclib::model::name::HasName;
use flowclib::model::name::Name;
use flowclib::model::process::Process::{FlowProcess, FunctionProcess};
use flowclib::model::route::HasRoute;
use flowclib::model::route::Route;
use flowrlib::input::InputInitializer::OneTime;
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
#[test]
fn args() {
    let meta_provider = MetaProvider {};
    let path = helper::absolute_file_url_from_relative_path("flowc/tests/test-flows/args.toml");
    let process = loader::load(&path, &meta_provider).unwrap();
    if let FlowProcess(ref flow) = process {
        let _tables = compile::compile(flow).unwrap();
    } else {
        assert!(false, "Process loaded was not a flow");
    }
}

#[test]
fn object_to_array_connection() {
    let meta_provider = MetaProvider {};
    let path = helper::absolute_file_url_from_relative_path("flowc/tests/test-flows/object_to_array_connection.toml");
    let process = loader::load(&path, &meta_provider).unwrap();
    if let FlowProcess(ref flow) = process {
        let _tables = compile::compile(flow).unwrap();
    } else {
        assert!(false, "Process loaded was not a flow");
    }
}

#[test]
fn context_with_io() {
    let meta_provider = MetaProvider {};
    let path = helper::absolute_file_url_from_relative_path("flowc/tests/test-flows/context_with_io.toml");
    let process = loader::load(&path, &meta_provider).unwrap();
    if let FlowProcess(ref flow) = process {
        let tables = compile::compile(flow);
        match tables {
            Ok(_) => {
                // flow loaded, but has ios
                assert!(flow.inputs().as_ref().unwrap().len() > 0);
                assert!(flow.outputs().as_ref().unwrap().len() > 0);
            }
            Err(_) => { /* Error was detected and reported correctly and didn't crash */ }
        }
    } else {
        assert!(false, "Process loaded was not a flow");
    }
}

#[test]
fn same_name_input_and_output() {
    let meta_provider = MetaProvider {};
    let path = helper::absolute_file_url_from_relative_path("flowc/tests/test-flows/same-name-parent.toml");
    let process = loader::load(&path, &meta_provider).unwrap();
    if let FlowProcess(ref flow) = process {
        let tables = compile::compile(flow).unwrap();
        // If done correctly there should only be two connections
        // args -> buffer, and buffer -> print
        assert_eq!(4, tables.collapsed_connections.len());
    } else {
        assert!(false, "Process loaded was not a flow");
    }
}

#[test]
fn same_name_flow_ids() {
    let meta_provider = MetaProvider {};
    let path = helper::absolute_file_url_from_relative_path("flowc/tests/test-flows/same-name-parent.toml");
    let process = loader::load(&path, &meta_provider).unwrap();
    if let FlowProcess(ref flow) = process {
        let tables = compile::compile(flow).unwrap();

        // print function in context flow should have flow_id = 0
        let print_function = tables.functions.iter()
            .find(|f| f.alias() == &Name::from("print")).unwrap();
        assert_eq!(print_function.get_flow_id(), 0, "print function in context should have flow_id = 0");

        // buffer function in first child flow should have flow_id = 1
        let buffer_function = tables.functions.iter()
            .find(|f| f.route() == &Route::from("/parent/child/buffer")).unwrap();
        assert_eq!(buffer_function.get_flow_id(), 1);

        // buffer function in second child flow should have flow_id = 2
        let buffer2_function = tables.functions.iter()
            .find(|f| f.route() == &Route::from("/parent/child2/buffer")).unwrap();
        assert_eq!(buffer2_function.get_flow_id(), 2);
    } else {
        assert!(false, "Process loaded was not a flow");
    }
}

#[test]
fn double_connection() {
    let meta_provider = MetaProvider {};
    let path = helper::absolute_file_url_from_relative_path("flowc/tests/test-flows/double-connection.toml");
    let process = loader::load(&path, &meta_provider).unwrap();
    if let FlowProcess(ref flow) = process {
        let tables = compile::compile(flow);
        match tables {
            Ok(_) => assert!(false, "Process should not have loaded due to double connection"),
            Err(_) => { /* Error was detected and reported correctly and didn't crash */ }
        }
    } else {
        assert!(false, "Process loaded was not a flow");
    }
}

#[test]
fn connection_to_input_with_constant_initializer() {
    let meta_provider = MetaProvider {};
    let path = helper::absolute_file_url_from_relative_path("flowc/tests/test-flows/connect_to_constant.toml");
    let process = loader::load(&path, &meta_provider).unwrap();
    if let FlowProcess(ref flow) = process {
        let tables = compile::compile(flow);
        match tables {
            Ok(_) => assert!(false, "Process should not have loaded due to connection to input with a constant initializer"),
            Err(_) => { /* Error was detected and reported correctly and didn't crash */ }
        }
    } else {
        assert!(false, "Process loaded was not a flow");
    }
}

#[test]
fn dead_process_removed() {
    let meta_provider = MetaProvider {};
    let path = helper::absolute_file_url_from_relative_path("flowc/tests/test-flows/dead-process.toml");
    let process = loader::load(&path, &meta_provider).unwrap();
    if let FlowProcess(ref flow) = process {
        let tables = compile::compile(flow).unwrap();
        // Dead value should be removed - currently can't assume that args function can be removed
        assert_eq!(tables.functions.len(), 1, "Incorrect number of functions after optimization");
        assert_eq!(tables.functions.get(0).unwrap().get_id(), 0, "Function indexes do not start at 0");
        // And the connection to it also
        assert_eq!(tables.collapsed_connections.len(), 0, "Incorrect number of connections after optimization");
    } else {
        assert!(false, "Process loaded was not a flow");
    }
}

#[test]
fn dead_process_and_connected_process_removed() {
    let meta_provider = MetaProvider {};
    let path = helper::absolute_file_url_from_relative_path("flowc/tests/test-flows/dead-process-and-connected-process.toml");
    let process = loader::load(&path, &meta_provider).unwrap();
    if let FlowProcess(ref flow) = process {
        let tables = compile::compile(flow).unwrap();
        assert!(tables.functions.is_empty(), "Incorrect number of functions after optimization");
        // And the connection are all gone also
        assert_eq!(tables.collapsed_connections.len(), 0, "Incorrect number of connections after optimization");
    } else {
        assert!(false, "Process loaded was not a flow");
    }
}

#[test]
fn compile_echo_ok() {
    let meta_provider = MetaProvider {};
    let process = loader::load(&helper::absolute_file_url_from_relative_path("flowc/tests/test-flows/echo.toml"),
                               &meta_provider).unwrap();
    if let FlowProcess(ref flow) = process {
        let _tables = compile::compile(flow).unwrap();
    } else {
        assert!(false, "Process loaded was not a flow");
    }
}

#[test]
fn compiler_detects_unused_input() {
    let meta_provider = MetaProvider {};
    let process = loader::load(&helper::absolute_file_url_from_relative_path("flowc/tests/test-flows/unused_input.toml"),
                               &meta_provider).unwrap();
    if let FlowProcess(ref flow) = process {
        assert!(compile::compile(flow).is_err(), "Should not compile due to unused input");
    } else {
        assert!(false, "Process loaded was not a flow");
    }
}

#[test]
fn compile_double_connection() {
    let meta_provider = MetaProvider {};
    let process = loader::load(&helper::absolute_file_url_from_relative_path("flowc/tests/test-flows/double.toml"),
                               &meta_provider).unwrap();
    if let FlowProcess(ref flow) = process {
        assert!(compile::compile(flow).is_err(), "Should not compile due to a double connection to an input");
    } else {
        assert!(false, "Process loaded was not a flow");
    }
}

#[test]
fn compile_detects_connection_to_initialized_input() {
    let meta_provider = MetaProvider {};
    let process = loader::load(&helper::absolute_file_url_from_relative_path("flowc/tests/test-flows/connect_to_constant.toml"),
                               &meta_provider).unwrap();
    if let FlowProcess(ref flow) = process {
        assert!(compile::compile(flow).is_err(), "Should not compile due to connection to constant initialized input");
    } else {
        assert!(false, "Process loaded was not a flow");
    }
}

/*
    This tests that an initalizer on an input to a flow sub-process is passed into it and
    back out via a connection
*/
#[test]
fn flow_input_propogated_backout() {
    let meta_provider = MetaProvider {};
    // Relative path from project root to the test file
    let url = helper::absolute_file_url_from_relative_path("flowc/tests/test-flows/subflow_input_init.toml");

    match loader::load(&url, &meta_provider) {
        Ok(FlowProcess(context)) => {
            match compile::compile(&context) {
                Ok(_tables) => {}
                Err(error) => panic!("Couldn't compile the flow from test file at '{}'\n{}", url, error)
            }
        },
        Ok(FunctionProcess(_)) => panic!("Unexpected compile result from test file at '{}'", url),
        Err(error) => panic!("Couldn't load the flow from test file at '{}'.\n{}", url, error)
    }
}

/*
    This tests that an initalizer on an input to a flow process is passed onto a function in
    a sub-flow of that via a connection from the flow input to the function input
*/
#[test]
fn flow_input_initialized_and_propogated_to_function_in_subflow() {
    let meta_provider = MetaProvider {};
    // Relative path from project root to the test file
    let url = helper::absolute_file_url_from_relative_path("flowc/tests/test-flows/subflow_function_input_init.toml");

    match loader::load(&url, &meta_provider) {
        Ok(FlowProcess(context)) => {
            let tables = compile::compile(&context).unwrap();

            match tables.functions.iter().find(|&f| f.route() == &Route::from("/subflow_function_input_init/sequence/compare")) {
                Some(compare_switch_function) => {
                    if let Some(inputs) = compare_switch_function.get_inputs() {
                        let in_input = inputs.get(1).unwrap();
                        assert_eq!(Name::from("right"), *in_input.alias(), "Input's name is not 'right' as expected");
                        let initial_value = in_input.get_initializer();
                        match initial_value {
                            Some(OneTime(one_time)) => assert_eq!(one_time.once, 1), // PASS
                            _ => panic!("Initializer should have been a OneTime initializer, was {:?}", initial_value)
                        }
                    } else {
                        panic!("Could not find any inputs");
                    }
                }
                None => panic!("Could not find function at route '/subflow_function_input_init/sequence/compare'")
            }
        }
        _ => panic!("Couldn't load the flow from test file at '{}'", url)
    }
}
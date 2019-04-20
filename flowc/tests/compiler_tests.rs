extern crate flowclib;
extern crate flowrlib;
extern crate provider;
extern crate url;

use std::env;

use flowclib::compiler::compile;
use flowclib::compiler::loader;
use flowclib::model::process::Process::FlowProcess;
use url::Url;

use provider::content::provider::MetaProvider;

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

// Helper function for tests
fn url_from_rel_path(path: &str) -> String {
    let cwd = Url::from_file_path(env::current_dir().unwrap()).unwrap();
    let source_file = cwd.join(file!()).unwrap();
    let file = source_file.join(path).unwrap();
    file.to_string()
}

fn set_flow_lib_path() {
    let mut parent_dir = std::env::current_dir().unwrap();
    parent_dir.pop();
    println!("Set 'FLOW_LIB_PATH' to '{}'", parent_dir.to_string_lossy().to_string());
    env::set_var("FLOW_LIB_PATH", parent_dir.to_string_lossy().to_string());
}

#[test]
fn args() {
    set_flow_lib_path();
    let meta_provider = MetaProvider {};
    let path = url_from_rel_path("test-flows/args.toml");
    let process = loader::load_context(&path, &meta_provider).unwrap();
    if let FlowProcess(ref flow) = process {
        let _tables = compile::compile(flow).unwrap();
    } else {
        assert!(false, "Process loaded was not a flow");
    }
}

#[test]
fn context_with_io() {
    set_flow_lib_path();
    let meta_provider = MetaProvider {};
    let path = url_from_rel_path("test-flows/context_with_io.toml");
    let process = loader::load_context(&path, &meta_provider).unwrap();
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
fn same_name_input_and_output() {
    set_flow_lib_path();
    let meta_provider = MetaProvider {};
    let path = url_from_rel_path("test-flows/same-name-parent.toml");
    let process = loader::load_context(&path, &meta_provider).unwrap();
    if let FlowProcess(ref flow) = process {
        let tables = compile::compile(flow).unwrap();
        // If done correctly there should only be two connections
        // args -> buffer, and buffer -> print
        assert_eq!(2, tables.collapsed_connections.len());
    } else {
        assert!(false, "Process loaded was not a flow");
    }
}

#[test]
fn double_connection() {
    set_flow_lib_path();
    let meta_provider = MetaProvider {};
    let path = url_from_rel_path("test-flows/double-connection.toml");
    let process = loader::load_context(&path, &meta_provider).unwrap();
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
fn dead_process_removed() {
    set_flow_lib_path();
    let meta_provider = MetaProvider {};
    let path = url_from_rel_path("test-flows/dead-process.toml");
    let process = loader::load_context(&path, &meta_provider).unwrap();
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
    set_flow_lib_path();
    let meta_provider = MetaProvider {};
    let path = url_from_rel_path("test-flows/dead-process-and-connected-process.toml");
    let process = loader::load_context(&path, &meta_provider).unwrap();
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
    set_flow_lib_path();
    let meta_provider = MetaProvider {};
    let process = loader::load_context(&url_from_rel_path("test-flows/echo.toml"),
                                       &meta_provider).unwrap();
    if let FlowProcess(ref flow) = process {
        let _tables = compile::compile(flow).unwrap();
    } else {
        assert!(false, "Process loaded was not a flow");
    }
}

#[test]
fn compiler_detects_unused_input() {
    set_flow_lib_path();
    let meta_provider = MetaProvider {};
    let process = loader::load_context(&url_from_rel_path("test-flows/unused_input.toml"),
                                       &meta_provider).unwrap();
    if let FlowProcess(ref flow) = process {
        assert!(compile::compile(flow).is_err(), "Should not compile due to unused input");
    } else {
        assert!(false, "Process loaded was not a flow");
    }
}

#[test]
fn compile_double_connection() {
    set_flow_lib_path();
    let meta_provider = MetaProvider {};
    let process = loader::load_context(&url_from_rel_path("test-flows/double.toml"),
                                       &meta_provider).unwrap();
    if let FlowProcess(ref flow) = process {
        assert!(compile::compile(flow).is_err(), "Should not compile due to a double connection to an input");
    } else {
        assert!(false, "Process loaded was not a flow");
    }
}

#[test]
fn compile_detects_connection_to_initialized_input() {
    set_flow_lib_path();
    let meta_provider = MetaProvider {};
    let process = loader::load_context(&url_from_rel_path("test-flows/connect_to_constant.toml"),
                                       &meta_provider).unwrap();
    if let FlowProcess(ref flow) = process {
        assert!(compile::compile(flow).is_err(), "Should not compile due to connection to constant initialized input");
    } else {
        assert!(false, "Process loaded was not a flow");
    }
}
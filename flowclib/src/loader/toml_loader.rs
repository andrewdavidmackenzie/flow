use toml;
use loader::loader::Loader;
use model::process::Process;

pub struct FlowTomelLoader;

impl Loader for FlowTomelLoader {
    fn load_process(&self, contents: &str) -> Result<Process, String> {
        toml::from_str(contents).map_err(|e| format!("{}", e))
    }
}

#[test]
fn simple_context_loads() {
    let flow_description = "\
        flow = 'hello-world-simple-toml'

        [[value]]
        name = 'message'
        type = 'String'

        [[process]]
        alias = 'print'
        source = 'terminal.toml'

        [[connection]]
        name = 'message'
        from = 'value/message'
        to = 'function/print/stdout'
    ";

    let toml = FlowTomelLoader {};
    toml.load_process(flow_description).unwrap();
}

#[test]
fn default_optional_values() {
    use super::super::model::flow::Flow;
    use super::super::model::process::Process::FlowProcess;
    let flow_description = "\
        flow = 'test'
    ";

    let toml = FlowTomelLoader {};
    match toml.load_process(flow_description).unwrap() {
        FlowProcess(flow) => {
            assert_eq!(flow.version, Flow::default_version());
            assert_eq!(flow.author_name, Flow::default_author());
            assert_eq!(flow.author_email, Flow::default_email());
        },
        _ => assert!(false)
    }
}

#[test]
fn flow_has_optional_values() {
    use super::super::model::process::Process::FlowProcess;
    let flow_description = "\
        flow = 'test'
        version = '1.1.1'
        author_name = 'tester'
        author_email = 'tester@test.com'
    ";

    let toml = FlowTomelLoader {};
    match toml.load_process(flow_description).unwrap() {
        FlowProcess(flow) => {
            assert_eq!(flow.version, "1.1.1".to_string());
            assert_eq!(flow.author_name, "tester".to_string());
            assert_eq!(flow.author_email, "tester@test.com".to_string());
        },
        _ => assert!(false)
    }
}

#[test]
fn flow_with_function_from_lib() {
    let flow_description = "\
        flow = 'use-library-function'

        [[process]]
        alias = 'print'
        source = 'lib://flowstdlib/stdio/stdout.toml'
    ";

    let toml = FlowTomelLoader {};
    toml.load_process(flow_description).unwrap();
}

#[test]
#[should_panic]
fn flow_with_unknown_lib_key() {
    let flow_description = "\
        name = 'use-library-function'

        [[process]]
        alias = 'print'
        lib = 'lib://fakelib/stdio/stdout.toml'
    ";

    let toml = FlowTomelLoader {};
    toml.load_process(flow_description).unwrap();
}

#[test]
#[should_panic]
fn flow_with_function_without_source() {
    let flow_description = "\
        name = 'use-library-function'

        [[process]]
        alias = 'print'
    ";

    let toml = FlowTomelLoader {};
    toml.load_process(flow_description).unwrap();
}

#[test]
#[should_panic]
fn load_fails_if_no_name() {
    let flow_description = "\
        [[value]]
        name = 'message'
        type = 'String'
        init = Hello World!'
    ";

    let toml = FlowTomelLoader {};
    toml.load_process(flow_description).unwrap();
}

#[test]
fn function_parses() {
    let function_definition = "\
function = 'stdout'

[[input]]
name = 'stdout'
type = 'String'";

    let toml = FlowTomelLoader {};
    toml.load_process(function_definition).unwrap();
}

#[test]
#[should_panic]
fn function_lacks_name() {
    let function_definition = "\
[[input]]
name = 'stdout'
type = 'String'";

    let toml = FlowTomelLoader {};
    toml.load_process(function_definition).unwrap();
}
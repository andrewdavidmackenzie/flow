use toml;
use loader::loader::Loader;
use model::flow::Flow;
use model::function::Function;

pub struct FlowTomelLoader;

impl Loader for FlowTomelLoader {
    fn load_flow(&self, contents: &str) -> Result<Flow, String> {
        toml::from_str(contents).map_err(|e| format!("{}", e))
    }

    fn load_function(&self, contents: &str) -> Result<Function, String> {
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

        [[function]]
        alias = 'print'
        source = 'terminal.toml'

        [[connection]]
        name = 'message'
        from = 'value/message'
        to = 'function/print/stdout'
    ";

    let toml = FlowTomelLoader {};
    toml.load_flow(flow_description).unwrap();
}

#[test]
fn flow_with_function_from_lib() {
    let flow_description = "\
        flow = 'use-library-function'

        [[function]]
        alias = 'print'
        source = 'lib://flowstdlib/stdio/stdout.toml'
    ";

    let toml = FlowTomelLoader {};
    toml.load_flow(flow_description).unwrap();
}

#[test]
#[should_panic]
fn flow_with_unknown_lib_key() {
    let flow_description = "\
        name = 'use-library-function'

        [[function]]
        alias = 'print'
        lib = 'lib://fakelib/stdio/stdout.toml'
    ";

    let toml = FlowTomelLoader {};
    toml.load_flow(flow_description).unwrap();
}

#[test]
#[should_panic]
fn flow_with_function_without_source() {
    let flow_description = "\
        name = 'use-library-function'

        [[function]]
        alias = 'print'
    ";

    let toml = FlowTomelLoader {};
    toml.load_flow(flow_description).unwrap();
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
    toml.load_flow(flow_description).unwrap();
}

#[test]
fn function_parses() {
    let function_definition = "\
function = 'stdout'

[[input]]
name = 'stdout'
type = 'String'";

    let toml = FlowTomelLoader {};
    toml.load_function(function_definition).unwrap();
}

#[test]
#[should_panic]
fn function_lacks_name() {
    let function_definition = "\
[[input]]
name = 'stdout'
type = 'String'";

    let toml = FlowTomelLoader {};
    toml.load_function(function_definition).unwrap();
}
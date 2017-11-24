use toml;

use loader::loader::Loader;
use description::flow::Flow;
use description::function::Function;

pub struct FlowTomelLoader {}

impl Loader for FlowTomelLoader {
    fn load_flow(&self, contents: &str) -> Result<Flow, String> {
        match toml::from_str(contents) {
            Ok(flow) => Ok(flow),
            Err(e) => Err(format!("{}", e))
        }
    }

    fn load_function(&self, contents: &str) -> Result<Function, String> {
        match toml::from_str(contents) {
            Ok(function) => Ok(function),
            Err(e) => Err(format!("{}", e))
        }
    }
}

#[test]
fn hello_world_simple_toml_context_loads() {
    let flow_description = "\
        name = 'hello-world-simple-toml'

        [[value]]
        name = 'message'
        datatype = 'String'
        value = 'Hello World!'

        [[function]]
        name = 'print'
        source = 'terminal.toml'

        [[connection]]
        name = 'message'
        from = 'value/message'
        to = 'function/print/stdout'
    ";

    let toml = FlowTomelLoader{};
    match toml.load_flow(flow_description) {
        Ok(_) => {}
        Err(error) => {
            eprintln!("{}", error);
            assert!(false)
        },
    }
}

#[test]
fn load_fails_if_no_name() {
    let flow_description = "\
        [[value]]
        name = 'message'
        datatype = 'String'
        value = Hello World!'
    ";

    let toml = FlowTomelLoader{};
    match toml.load_flow(flow_description) {
        Ok(_) => assert!(false),
        _ => {}
    }
}

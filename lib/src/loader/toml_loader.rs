use toml;

use loader::loader::Loader;
use description::flow::Flow;

pub struct FlowTomelLoader {}

/*
 load the toml file which should contain a flow description
 */
impl Loader for FlowTomelLoader {
    // TODO define our own errors types? so we can return errors from lower down directly
    fn load_flow(&self, contents: &str) -> Result<Flow, String> {
        match toml::from_str(contents) {
            Ok(flow) => Ok(flow),
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
        Ok(flow) => {}
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
        Ok(flow) => assert!(false),
        _ => {}
    }
}

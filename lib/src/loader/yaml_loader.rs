extern crate yaml_rust;

use description::flow::Flow;
use loader::loader::Loader;
use std::path::PathBuf;
use description::function::Function;

pub struct FlowYamlLoader {}

impl Loader for FlowYamlLoader {
    // TODO define our own errors types? so we can return errors from lower down directly
    fn load_flow(&self, contents: &str) -> Result<Flow, String> {
//        let docs = YamlLoader::load_from_str(&contents).unwrap();
//        let doc = &docs[0];

        let flow =
            Flow {
                source: PathBuf::from("fake"),
                name: "fake".to_string(),
                flow: None,
                connection: None,
                input: None,
                output: None,
                function: None,
                value: None,
            };

        Ok(flow)
    }

    fn load_function(&self, contents: &str) -> Result<Function, String> {
//        let docs = YamlLoader::load_from_str(&contents).unwrap();
//        let doc = &docs[0];

        let function = Function {
            name: "fake".to_string(),
            input: None,
            output: None,
            implementation: None
        };

        Ok(function)
    }
}

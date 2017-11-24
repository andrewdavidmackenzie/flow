extern crate yaml_rust;

use self::yaml_rust::YamlLoader;
use description::flow::Flow;
use loader::loader::Loader;
use std::path::PathBuf;

pub struct FlowYamlLoader {}

/*
 laod the yaml file(s)
 */
impl Loader for FlowYamlLoader {
    // TODO define our own errors types? so we can return errors from lower down directly
    fn load_flow(&self, contents: &str) -> Result<Flow, String> {
        let docs = YamlLoader::load_from_str(&contents).unwrap();
        let doc = &docs[0];

        let flow =
            Flow {
                source: PathBuf::from("fake"),
                name: "fake".to_string(),
                flow: None,
                connection: None,
                io: None,
                function: None,
                value: None,
                flows: vec ! ()
                /*
                entities: entities,
                values: values,
                connection_set: connection_set,
                */
            };

        Ok(flow)
    }
}

use model::flow::Flow;
use loader::loader::Loader;
use model::function::Function;
use url::Url;

pub struct FlowYamlLoader;

impl Loader for FlowYamlLoader {
    // TODO define our own errors types? so we can return errors from lower down directly
    fn load_flow(&self, _contents: &str) -> Result<Flow, String> {
//        let docs = YamlLoader::load_from_str(&contents).unwrap();
//        let doc = &docs[0];

        let flow = Flow::new("name".to_string(),"alias".to_string(), Url::parse("fake").unwrap(),
                             "fake/fake".to_string(),
                             None, None, None, None, None, None,
                             vec!());

        Ok(flow)
    }

    fn load_function(&self, _contents: &str) -> Result<Function, String> {
//        let docs = YamlLoader::load_from_str(&contents).unwrap();
//        let doc = &docs[0];

        let function = Function::default();

        Ok(function)
    }
}

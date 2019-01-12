use loader::loader::Loader;
use url::Url;
use model::flow::Flow;
use model::process::Process;

pub struct FlowYamlLoader;

impl Loader for FlowYamlLoader {
    fn load_process(&self, _contents: &str) -> Result<Process, String> {
//        let docs = YamlLoader::load_from_str(&contents).unwrap();
//        let doc = &docs[0];

        let flow = Flow::new("name".to_string(),"alias".to_string(), Url::parse("fake").unwrap(),
                             "fake/fake".to_string(),
                             None, None, None, None, None,
                             vec!());

        Ok(Process::FlowProcess(flow))
    }
}

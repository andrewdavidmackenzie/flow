use loader::loader::Deserializer;
use model::flow::Flow;
use model::process::Process;

pub struct FlowYamlLoader;

impl Deserializer for FlowYamlLoader {
    fn deserialize(&self, _contents: &str) -> Result<Process, String> {
//        let docs = YamlLoader::load_from_str(&contents).unwrap();
//        let doc = &docs[0];

        let flow = Flow::default();

        Ok(Process::FlowProcess(flow))
    }
}

use model::flow::Flow;
use model::function::Function;

#[derive(Deserialize, Clone)]
#[serde(untagged)]
pub enum Process {
    FlowProcess(Flow),
    FunctionProcess(Function)
}

impl Default for Process {
    fn default() -> Process {
        Process::FlowProcess(Flow::default())
    }
}
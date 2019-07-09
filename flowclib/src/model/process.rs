use crate::model::flow::Flow;
use crate::model::function::Function;

#[derive(Serialize, Deserialize, Debug, Clone)]
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
use serde_derive::{Deserialize, Serialize};

use crate::model::flow::Flow;
use crate::model::function::Function;
use crate::model::name::{HasName, Name};
use crate::model::route::{HasRoute, Route};

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

impl HasName for Process {
    fn name(&self) -> &Name {
        match self {
            Process::FlowProcess(flow) => flow.name(),
            Process::FunctionProcess(function) => function.name()
        }
    }

    fn alias(&self) -> &Name {
        match self {
            Process::FlowProcess(flow) => flow.alias(),
            Process::FunctionProcess(function) => function.alias()
        }
    }
}

impl HasRoute for Process {
    fn route(&self) -> &Route {
        match self {
            Process::FlowProcess(ref flow) => flow.route(),
            Process::FunctionProcess(ref function) => function.route()
        }
    }

    fn route_mut(&mut self) -> &mut Route {
        match self {
            Process::FlowProcess(ref mut flow) => flow.route_mut(),
            Process::FunctionProcess(ref mut function) => function.route_mut()
        }
    }
}
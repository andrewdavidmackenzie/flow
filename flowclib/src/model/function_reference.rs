use std::fmt;

use model::name::Name;
use model::name::HasName;
use model::route::Route;
use model::route::HasRoute;
use loader::loader::Validate;
use model::process_reference::Process;

// This structure is (optionally) found as part of a flow file - inline in the description
#[derive(Deserialize)]
pub struct FunctionReference {
    alias: Name,
    pub source: String,
    #[serde(skip_deserializing)]
    pub process: Process,
}

impl HasName for FunctionReference {
    fn name(&self) -> &Name { &self.alias }
    fn alias(&self) -> &Name { &self.alias }
}

impl HasRoute for FunctionReference {
    fn route(&self) -> &Route {
        match self.process {
            Process::FlowProcess(ref flow) => {
                flow.route()
            }
            Process::FunctionProcess(ref function) => {
                function.route()
            }
        }
    }
}

impl Validate for FunctionReference {
    fn validate(&self) -> Result<(), String> {
        self.alias.validate()
    }
}

impl fmt::Display for FunctionReference {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\t\t\t\talias: \t{}\n\t\t\t\t\timplementation:\n\t\t\t\t\tsource: \t{}\n",
               self.alias, self.source)
    }
}
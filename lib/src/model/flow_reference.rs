use model::name::Name;
use model::name::HasName;
use model::name::HasRoute;
use model::flow::Flow;
use loader::loader::Validate;

use std::fmt;

#[derive(Default, Deserialize, Debug)]
pub struct FlowReference {
    #[serde(rename = "name")]
    pub alias: Name,
    pub source: String,
    #[serde(skip_deserializing)]
    pub flow: Flow
}

// TODO figure out how to have this derived automatically for types needing it
impl HasName for FlowReference {
    fn name(&self) -> &str {
        &self.alias[..]
    }
}

impl HasRoute for FlowReference {
    fn route(&self) -> &str {
        &self.flow.route[..]
    }
}

impl Validate for FlowReference {
    fn validate(&self) -> Result<(), String> {
        self.alias.validate()
        // Pretty much anything is a valid PathBuf - so not sure how to validate source...
    }
}

impl fmt::Display for FlowReference {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\t\t\t\talias: {}\n\t\t\t\t\tsource: {}\n",
               self.alias, self.source)
    }
}
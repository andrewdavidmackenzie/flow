use model::name::Name;
use model::name::HasName;
use model::route::Route;
use model::route::HasRoute;
use model::flow::Flow;
use loader::loader::Validate;
use std::fmt;
use url::Url;

#[derive(Deserialize)]
pub struct FlowReference {
    alias: Name,
    pub source: String,
    #[serde(skip_deserializing, default = "FlowReference::default_url")]
    pub source_url: Url,
    #[serde(skip_deserializing)]
    pub flow: Flow
}

impl HasName for FlowReference {
    fn name(&self) -> &Name {  &self.alias  }
    fn alias(&self) -> &Name {  &self.alias  }
}

impl HasRoute for FlowReference {
    fn route(&self) -> &Route {
        &self.flow.route
    }
}

impl Validate for FlowReference {
    fn validate(&self) -> Result<(), String> {
        self.alias.validate()
            // TODO subsititure Source for Url and have it implement Validate trait
//        self.source.validate()
    }
}

impl fmt::Display for FlowReference {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\t\t\t\talias: {}\n\t\t\t\t\tsource: {}\n",
               self.alias, self.source)
    }
}

impl FlowReference {
    fn default_url() -> Url {
        Url::parse("file::///").unwrap()
    }
}
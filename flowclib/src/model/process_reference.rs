use model::name::Name;
use model::name::HasName;
use model::route::Route;
use model::route::HasRoute;
use model::process::Process;
use loader::loader::Validate;
use std::fmt;
use url::Url;

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct ProcessReference {
    pub alias: Name,
    pub source: String,
    #[serde(skip_deserializing, default = "ProcessReference::default_url")]
    pub source_url: Url,
    #[serde(skip_deserializing)]
    pub process: Process
}

impl HasName for ProcessReference {
    fn name(&self) -> &Name { &self.alias }
    fn alias(&self) -> &Name { &self.alias }
}

impl HasRoute for ProcessReference {
    fn route(&self) -> &Route {
        match self.process {
            Process::FlowProcess(ref flow) => {
                flow.route()
            },
            Process::FunctionProcess(ref function) => {
                function.route()
            }
        }
    }
}

impl Validate for ProcessReference {
    fn validate(&self) -> Result<(), String> {
        self.alias.validate()
    }
}

impl fmt::Display for ProcessReference {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\t\t\t\talias: {}\n\t\t\t\t\tsource: {}\n\t\t\t\t\tURL: {}\n",
               self.alias, self.source, self.source_url)
    }
}

impl ProcessReference {
    fn default_url() -> Url {
        Url::parse("file::///").unwrap()
    }
}


#[cfg(test)]
mod test {
    use super::ProcessReference;

    #[test]
    fn deserialize_simple() {
        let input_str = "
        alias = 'other'
        source = 'other.toml'
        ";

        let _reference: ProcessReference = toml::from_str(input_str).unwrap();
    }

    #[test]
    #[should_panic]
    fn deserialize_extra_field_fails() {
        let input_str = "
        alias = 'other'
        source = 'other.toml'
        foo = 'extra token'
        ";

        let _reference: ProcessReference = toml::from_str(input_str).unwrap();
    }
}
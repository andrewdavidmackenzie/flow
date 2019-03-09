use serde_json::Value as JsonValue;
use model::name::Name;
use model::name::HasName;
use model::route::Route;
use model::route::HasRoute;
use model::process::Process;
use loader::loader::Validate;
use std::fmt;
use std::collections::HashMap;

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct ProcessReference {
    pub alias: Name,
    pub source: String,
    #[serde(rename = "input")]
    pub initializations: Option<HashMap<String, JsonValue>>,
    // input_name (String) = initial_value (JsonValue)
    #[serde(skip_deserializing, default = "ProcessReference::default_url")]
    pub source_url: String,
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
    fn default_url() -> String {
        "file::///".to_string()
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
    fn deserialize_with_input_initialization() {
        let input_str = "
        alias = 'other'
        source = 'other.toml'
        input.input1 = 1
        ";

        let reference: ProcessReference = toml::from_str(input_str).unwrap();
        let initialized_inputs = reference.initializations.unwrap();
        assert_eq!(initialized_inputs.len(), 1, "Incorrect number of Input initializations parsed");
        assert_eq!(initialized_inputs.get("input1").unwrap(), 1, "input1 should be initialized to 1");
    }

    #[test]
    fn deserialize_with_multiple_input_initialization() {
        let input_str = "
        alias = 'other'
        source = 'other.toml'
        input.input1 = 1
        input.input2 = 'hello'
        ";

        let reference: ProcessReference = toml::from_str(input_str).unwrap();
        let initialized_inputs = reference.initializations.unwrap();
        assert_eq!(initialized_inputs.len(), 2, "Incorrect number of Input initializations parsed");
        assert_eq!(initialized_inputs.get("input1").unwrap(), 1, "input1 should be initialized to 1");
        assert_eq!(initialized_inputs.get("input2").unwrap(), "hello",
        "input2 should be initialized to 'hello'");
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
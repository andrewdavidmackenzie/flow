use serde_derive::{Deserialize, Serialize};

use crate::model::flow::Flow;
use crate::model::function_definition::FunctionDefinition;
use crate::model::name::{HasName, Name};
use crate::model::route::{HasRoute, Route};

/// Process is an enum that may contain a Flow or a Function
#[derive(Serialize, Deserialize, Debug, Clone)]
#[allow(clippy::large_enum_variant)]
#[serde(untagged)]
pub enum Process {
    /// The process is actually a `Flow`
    FlowProcess(Flow),
    /// The process is actually a `Function`
    FunctionProcess(FunctionDefinition),
}

impl HasName for Process {
    fn name(&self) -> &Name {
        match self {
            Process::FlowProcess(flow) => flow.name(),
            Process::FunctionProcess(function) => function.name(),
        }
    }

    fn alias(&self) -> &Name {
        match self {
            Process::FlowProcess(flow) => flow.alias(),
            Process::FunctionProcess(function) => function.alias(),
        }
    }
}

impl HasRoute for Process {
    fn route(&self) -> &Route {
        match self {
            Process::FlowProcess(ref flow) => flow.route(),
            Process::FunctionProcess(ref function) => function.route(),
        }
    }

    fn route_mut(&mut self) -> &mut Route {
        match self {
            Process::FlowProcess(ref mut flow) => flow.route_mut(),
            Process::FunctionProcess(ref mut function) => function.route_mut(),
        }
    }
}

#[cfg(test)]
mod test {
    use url::Url;

    use flowcore::deserializers::deserializer::get_deserializer;
    use flowcore::errors::*;

    use crate::model::process::Process;
    use crate::model::process::Process::FlowProcess;

    fn toml_from_str(content: &str) -> Result<Process> {
        let url = Url::parse("file:///fake.toml").expect("Could not parse URL");
        let deserializer = get_deserializer::<Process>(&url).expect("Could not get deserializer");
        deserializer.deserialize(content, Some(&url))
    }

    fn yaml_from_str(content: &str) -> Result<Process> {
        let url = Url::parse("file:///fake.yaml").expect("Could not parse URL");
        let deserializer = get_deserializer::<Process>(&url).expect("Could not get deserializer");
        deserializer.deserialize(content, Some(&url))
    }

    fn json_from_str(content: &str) -> Result<Process> {
        let url = Url::parse("file:///fake.json").expect("Could not parse URL");
        let deserializer = get_deserializer::<Process>(&url).expect("Could not get deserializer");
        deserializer.deserialize(content, Some(&url))
    }

    #[test]
    fn flow_with_partial_metadata() {
        let flow_description = "
flow: hello-world-simple-toml

metadata:
  version: '1.1.1'
  authors: ['unknown <unknown@unknown.com>']
";

        match yaml_from_str(&flow_description.replace("'", "\"")) {
            Ok(FlowProcess(flow)) => {
                assert_eq!(flow.metadata.name, String::default());
                assert_eq!(flow.metadata.version, "1.1.1".to_string());
                assert_eq!(
                    flow.metadata.authors,
                    vec!("unknown <unknown@unknown.com>".to_string())
                );
            }
            _ => panic!("Deserialization didn't detect a flow"),
        }
    }

    #[test]
    fn simple_context_loads() {
        let flow_description = "\
        flow = 'hello-world-simple-toml'

        [[process]]
        alias = 'input'
        source = 'lib://context/stdio/stdin.toml'

        [[process]]
        alias = 'print'
        source = 'lib://context/stdio/stdout.toml'

        [[connection]]
        from = 'input'
        to = 'print'
    ";

        assert!(toml_from_str(flow_description).is_ok());
    }

    #[test]
    fn flow_errors_on_unknown_fields() {
        let flow_description = "\
        flow = 'hello-world-simple-toml'

        foo = 'true'

        [[bar]]
        bar = 'true'
    ";

        assert!(toml_from_str(flow_description).is_err());
    }

    #[test]
    fn function_errors_on_unknown_fields() {
        let flow_description = "\
        function = 'hello-world-simple-toml'
        [[output]]

        foo = 'true'

        [[bar]]
        bar = 'true'
    ";

        assert!(toml_from_str(flow_description).is_err());
    }

    #[test]
    fn default_optional_values() {
        let flow_description = "\
        flow = 'test'
    ";

        match toml_from_str(flow_description) {
            Ok(FlowProcess(flow)) => {
                assert_eq!(flow.metadata.version, String::default());
                assert_eq!(flow.metadata.authors, Vec::<String>::default());
            }
            _ => panic!(),
        }
    }

    #[test]
    fn flow_has_metadata() {
        let flow_description = "\
flow = 'test'

[metadata]
name = \"me\"
version = \"1.1.1\"
description = \"ok\"
authors = [\"Andrew <andrew@foo.com>\"]
    ";

        match toml_from_str(flow_description) {
            Ok(FlowProcess(flow)) => {
                assert_eq!(flow.metadata.name, "me".to_string());
                assert_eq!(flow.metadata.version, "1.1.1".to_string());
                assert_eq!(flow.metadata.description, "ok".to_string());
                assert_eq!(
                    flow.metadata.authors,
                    vec!("Andrew <andrew@foo.com>".to_string())
                );
            }
            Ok(_) => panic!("Deserialization didn't detect a flow"),
            Err(e) => panic!("Deserialization error: {:?}", e),
        }
    }

    #[test]
    fn flow_has_partial_metadata() {
        let flow_description = "\
flow = 'test'

[metadata]
version = \"1.1.1\"
    ";

        match toml_from_str(flow_description) {
            Ok(FlowProcess(flow)) => {
                assert_eq!(flow.metadata.name, String::default());
                assert_eq!(flow.metadata.version, "1.1.1".to_string());
                assert_eq!(flow.metadata.description, String::default());
            }
            Ok(_) => panic!("Deserialization didn't detect a flow"),
            Err(e) => panic!("Deserialization error: {:?}", e),
        }
    }

    #[test]
    fn flow_with_function_from_lib() {
        let flow_description = "\
        flow = 'use-library-function'

        [[process]]
        alias = 'print'
        source = 'lib://context/stdio/stdout.toml'
    ";

        assert!(toml_from_str(flow_description).is_ok());
    }

    #[test]
    fn flow_with_unknown_lib_key() {
        let flow_description = "\
        flow = 'use-library-function'

        [[process]]
        alias = 'print'
        lib = 'lib://fake/stdio/stdout.toml'
    ";

        assert!(toml_from_str(flow_description).is_err());
    }

    #[test]
    fn flow_with_function_without_source() {
        let flow_description = "\
        flow = 'use-library-function'

        [[process]]
        alias = 'print'
    ";

        assert!(toml_from_str(flow_description).is_err());
    }

    #[test]
    fn load_fails_if_no_alias() {
        let flow_description = "\
        [[process]]
        source = 'lib://context/stdio/stdout.toml'
    ";

        assert!(toml_from_str(flow_description).is_err());
    }

    #[test]
    fn function_parses() {
        let function_definition = "\
function = 'stdout'
source = 'stdout.rs'
[[input]]
name = 'stdout'
type = 'String'";

        assert!(toml_from_str(function_definition).is_ok());
    }

    #[test]
    fn function_lacks_name() {
        let function_definition = "\
source = 'stdout.rs'
[[input]]
name = 'stdout'
type = 'String'";

        assert!(toml_from_str(function_definition).is_err());
    }

    #[test]
    fn function_lacks_implementation() {
        let function_definition = "\
function = 'stdout'
[[input]]
name = 'stdout'
type = 'String'";

        assert!(toml_from_str(function_definition).is_err());
    }

    #[test]
    fn simplest_context_loads() {
        let flow_description = "{
    'flow': 'hello-world-simple-toml',
    'process': [
        {
            'alias': 'print',
            'source': 'lib://context/stdio/stdout.toml',
            'input': {
                'default': {
                    'once': 'hello'
                }
            }
        }
    ]
}";

        let flow = json_from_str(&flow_description.replace("'", "\""));
        assert!(flow.is_ok());
    }

    #[test]
    fn simple_context_loads_from_json() {
        let flow_description = "{
    'flow': 'hello-world-simple-toml',
    'process': [
        {
            'alias': 'input',
            'source': 'lib://context/stdio/stdin.toml'
        },
        {
            'alias': 'print',
            'source': 'lib://context/stdio/stdout.toml'
        }
    ],
    'connection': [
        {
            'from': 'input/string',
            'to': 'print'
        }
    ]
}";

        let flow = json_from_str(&flow_description.replace("'", "\""));
        assert!(flow.is_ok());
    }

    #[test]
    fn invalid_context_fails() {
        let flow_description = "{
    'flow': 'hello-world-simple-toml',
    'process': [
        {
            'alias': 'message',
            'source': 'lib://context/stdio/stdin.toml'
        },
        {
            'alias': 'print',
            'source': 'lib://context/stdio/stdout.toml'
        }
    ],
    'connection': [
        {\
            'from': 'message'
        }
    ]
}";

        let flow = json_from_str(&flow_description.replace("'", "\""));
        assert!(flow.is_err());
    }
}

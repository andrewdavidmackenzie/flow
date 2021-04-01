use url::Url;

use crate::compiler::loader::Deserializer;
use crate::errors::*;
use crate::model::process::Process;

pub struct FlowTomlLoader;

// NOTE: Add one to make indexes one-based
impl Deserializer for FlowTomlLoader {
    fn deserialize(&self, contents: &str, url: Option<&Url>) -> Result<Process> {
        toml::from_str(contents).chain_err(|| {
            format!(
                "Error deserializing Toml from: '{}'",
                url.map_or("URL unknown".to_owned(), |u| u.to_string())
            )
        })
    }

    fn name(&self) -> &'static str {
        "Toml"
    }
}

#[cfg(test)]
mod test {
    use crate::compiler::loader::Deserializer;
    use crate::model::flow::Flow;
    use crate::model::process::Process::FlowProcess;

    use super::FlowTomlLoader;

    #[test]
    fn invalid_toml() {
        let deserializer = FlowTomlLoader {};

        if deserializer.deserialize("{}}}}f fake data ", None).is_ok() {
            panic!("Should not have parsed correctly as is invalid TOML");
        };
    }

    #[test]
    fn simple_context_loads() {
        let flow_description = "\
        flow = 'hello-world-simple-toml'

        [[process]]
        alias = 'message'
        source = 'lib://flowstdlib/data/buffer.toml'
        input.default = {once = 'hello'}

        [[process]]
        alias = 'print'
        source = 'lib://flowruntime/stdio/stdout.toml'

        [[connection]]
        from = 'message'
        to = 'print'
    ";

        let toml = FlowTomlLoader {};
        let flow = toml.deserialize(flow_description, None);
        assert!(flow.is_ok());
    }

    #[test]
    fn flow_errors_on_unknown_fields() {
        let flow_description = "\
        flow = 'hello-world-simple-toml'

        foo = 'true'

        [[bar]]
        bar = 'true'
    ";

        let toml = FlowTomlLoader {};
        assert!(toml.deserialize(flow_description, None).is_err());
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

        let toml = FlowTomlLoader {};
        assert!(toml.deserialize(flow_description, None).is_err());
    }

    #[test]
    fn default_optional_values() {
        let flow_description = "\
        flow = 'test'
    ";

        let toml = FlowTomlLoader {};
        match toml.deserialize(flow_description, None) {
            Ok(FlowProcess(flow)) => {
                assert_eq!(flow.version, Flow::default_version());
                assert_eq!(flow.authors, Flow::default_authors());
            }
            _ => panic!(),
        }
    }

    #[test]
    fn flow_has_optional_values() {
        let flow_description = "\
        flow = 'test'
        version = '1.1.1'
        authors = ['tester <tester@test.com>']
    ";

        let toml = FlowTomlLoader {};
        match toml.deserialize(flow_description, None) {
            Ok(FlowProcess(flow)) => {
                assert_eq!(flow.version, "1.1.1".to_string());
                assert_eq!(flow.authors, vec!("tester <tester@test.com>".to_string()));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn flow_with_function_from_lib() {
        let flow_description = "\
        flow = 'use-library-function'

        [[process]]
        alias = 'print'
        source = 'lib://flowstdlib/stdio/stdout.toml'
    ";

        let toml = FlowTomlLoader {};
        assert!(toml.deserialize(flow_description, None).is_ok());
    }

    #[test]
    fn flow_with_unknown_lib_key() {
        let flow_description = "\
        name = 'use-library-function'

        [[process]]
        alias = 'print'
        lib = 'lib://fake/stdio/stdout.toml'
    ";

        let toml = FlowTomlLoader {};
        assert!(toml.deserialize(flow_description, None).is_err());
    }

    #[test]
    fn flow_with_function_without_source() {
        let flow_description = "\
        name = 'use-library-function'

        [[process]]
        alias = 'print'
    ";

        let toml = FlowTomlLoader {};
        assert!(toml.deserialize(flow_description, None).is_err());
    }

    #[test]
    fn load_fails_if_no_alias() {
        let flow_description = "\
        [[process]]
        source = 'lib://flowstdlib/stdio/stdout.toml'
    ";

        let toml = FlowTomlLoader {};
        assert!(toml.deserialize(flow_description, None).is_err());
    }

    #[test]
    fn function_parses() {
        let function_definition = "\
function = 'stdout'
implementation = 'stdout.rs'
[[input]]
name = 'stdout'
type = 'String'";

        let toml = FlowTomlLoader {};
        assert!(toml.deserialize(function_definition, None).is_ok());
    }

    #[test]
    fn function_lacks_name() {
        let function_definition = "\
implementation = 'stdout.rs'
[[input]]
name = 'stdout'
type = 'String'";

        let toml = FlowTomlLoader {};
        assert!(toml.deserialize(function_definition, None).is_err());
    }

    #[test]
    fn function_lacks_implementation() {
        let function_definition = "\
function = 'stdout'
[[input]]
name = 'stdout'
type = 'String'";

        let toml = FlowTomlLoader {};
        assert!(toml.deserialize(function_definition, None).is_err());
    }
}

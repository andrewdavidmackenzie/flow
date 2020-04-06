use crate::compiler::loader::Deserializer;
use crate::errors::*;
use crate::model::process::Process;

pub struct FlowTomelLoader;

// NOTE: Add one to make indexes one-based
impl Deserializer for FlowTomelLoader {
    fn deserialize(&self, contents: &str, url: Option<&str>) -> Result<Process> {
        toml::from_str(contents)
            .chain_err(|| format!("Error deserializing Toml from: '{}'",
                                  url.or_else(|| { Some("URL unknown") } ).unwrap()))
    }

    fn name(&self) -> &'static str { "Toml" }
}

#[cfg(test)]
mod test {
    use crate::compiler::loader::Deserializer;
    use crate::model::flow::Flow;
    use crate::model::process::Process::FlowProcess;

    use super::FlowTomelLoader;

    #[test]
    fn invalid_toml() {
        let deserializer = FlowTomelLoader {};

        match deserializer.deserialize("{}}}}f dsdsadsa ", None) {
            Ok(_) => assert!(false, "Should not have parsed correctly as is invalid TOML"),
            Err(_) => assert!(true, "Should produce syntax error at (1,1)")
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

        let toml = FlowTomelLoader {};
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

        let toml = FlowTomelLoader {};
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

        let toml = FlowTomelLoader {};
        assert!(toml.deserialize(flow_description, None).is_err());
    }

    #[test]
    fn default_optional_values() {
        let flow_description = "\
        flow = 'test'
    ";

        let toml = FlowTomelLoader {};
        match toml.deserialize(flow_description, None) {
            Ok(FlowProcess(flow)) => {
                assert_eq!(flow.version, Flow::default_version());
                assert_eq!(flow.author_name, Flow::default_author());
                assert_eq!(flow.author_email, Flow::default_email());
            }
            _ => assert!(false)
        }
    }

    #[test]
    fn flow_has_optional_values() {
        let flow_description = "\
        flow = 'test'
        version = '1.1.1'
        author_name = 'tester'
        author_email = 'tester@test.com'
    ";

        let toml = FlowTomelLoader {};
        match toml.deserialize(flow_description, None) {
            Ok(FlowProcess(flow)) => {
                assert_eq!(flow.version, "1.1.1".to_string());
                assert_eq!(flow.author_name, "tester".to_string());
                assert_eq!(flow.author_email, "tester@test.com".to_string());
            }
            _ => assert!(false)
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

        let toml = FlowTomelLoader {};
        assert!(toml.deserialize(flow_description, None).is_ok());
    }

    #[test]
    fn flow_with_unknown_lib_key() {
        let flow_description = "\
        name = 'use-library-function'

        [[process]]
        alias = 'print'
        lib = 'lib://fakelib/stdio/stdout.toml'
    ";

        let toml = FlowTomelLoader {};
        assert!(toml.deserialize(flow_description, None).is_err());
    }

    #[test]
    fn flow_with_function_without_source() {
        let flow_description = "\
        name = 'use-library-function'

        [[process]]
        alias = 'print'
    ";

        let toml = FlowTomelLoader {};
        assert!(toml.deserialize(flow_description, None).is_err());
    }

    #[test]
    fn load_fails_if_no_alias() {
        let flow_description = "\
        [[process]]
        source = 'lib://flowstdlib/stdio/stdout.toml'
    ";

        let toml = FlowTomelLoader {};
        assert!(toml.deserialize(flow_description, None).is_err());
    }

    #[test]
    fn function_parses() {
        let function_definition = "\
function = 'stdout'

[[input]]
name = 'stdout'
type = 'String'";

        let toml = FlowTomelLoader {};
        assert!(toml.deserialize(function_definition, None).is_ok());
    }

    #[test]
    fn function_lacks_name() {
        let function_definition = "\
[[input]]
name = 'stdout'
type = 'String'";

        let toml = FlowTomelLoader {};
        assert!(toml.deserialize(function_definition, None).is_err());
    }
}
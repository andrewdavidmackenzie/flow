use std::marker::PhantomData;

use serde::Deserialize;
use url::Url;

use crate::compiler::loader::Deserializer;
use crate::errors::*;

pub struct TomlDeserializer<'a, T>
where
    T: Deserialize<'a>,
{
    t: PhantomData<&'a T>,
}

impl<'a, T> TomlDeserializer<'a, T>
where
    T: Deserialize<'a>,
{
    pub fn new() -> Self {
        TomlDeserializer { t: PhantomData }
    }
}

impl<'a, T> Deserializer<'a, T> for TomlDeserializer<'a, T>
where
    T: Deserialize<'a>,
{
    fn deserialize(&self, contents: &'a str, url: Option<&Url>) -> Result<T> {
        toml::from_str(contents).chain_err(|| {
            format!(
                "Error deserializing Toml from: '{}'",
                url.map_or("URL was None".to_owned(), |u| u.to_string())
            )
        })
    }

    fn name(&self) -> &str {
        "Toml"
    }
}

#[cfg(test)]
mod test {
    use toml::de::Error;

    use flowcore::flow_manifest::MetaData;

    use crate::compiler::loader::Deserializer;
    use crate::model::process::Process;
    use crate::model::process::Process::FlowProcess;

    use super::TomlDeserializer;

    #[test]
    fn invalid_toml() {
        let toml = TomlDeserializer::<Process>::new();
        if toml.deserialize("{}}}}f fake data ", None).is_ok() {
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

        let toml = TomlDeserializer::<Process>::new();
        assert!(toml.deserialize(flow_description, None).is_ok());
    }

    #[test]
    fn flow_errors_on_unknown_fields() {
        let flow_description = "\
        flow = 'hello-world-simple-toml'

        foo = 'true'

        [[bar]]
        bar = 'true'
    ";

        let toml = TomlDeserializer::<Process>::new();
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

        let toml = TomlDeserializer::<Process>::new();
        assert!(toml.deserialize(flow_description, None).is_err());
    }

    #[test]
    fn default_optional_values() {
        let flow_description = "\
        flow = 'test'
    ";

        let toml = TomlDeserializer::<Process>::new();
        match toml.deserialize(flow_description, None) {
            Ok(FlowProcess(flow)) => {
                assert_eq!(flow.metadata.version, String::default());
                assert_eq!(flow.metadata.authors, Vec::<String>::default());
            }
            _ => panic!(),
        }
    }

    #[test]
    fn metadata() {
        let metadata = "\
name = \"me\"
version = \"1.1.1\"
description = \"ok\"
authors = [\"Andrew <andrew@foo.com>\"]
    ";

        let result: Result<MetaData, Error> = toml::from_str(metadata);
        match result {
            Ok(md) => {
                assert_eq!(md.name, "me".to_string());
                assert_eq!(md.version, "1.1.1".to_string());
                assert_eq!(md.description, "ok".to_string());
                assert_eq!(md.authors, vec!("Andrew <andrew@foo.com>".to_string()));
            }
            Err(e) => panic!("Deserialization error: {:?}", e),
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

        let toml = TomlDeserializer::<Process>::new();
        match toml.deserialize(flow_description, None) {
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

        let toml = TomlDeserializer::<Process>::new();
        match toml.deserialize(flow_description, None) {
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
        source = 'lib://flowstdlib/stdio/stdout.toml'
    ";

        let toml = TomlDeserializer::<Process>::new();
        assert!(toml.deserialize(flow_description, None).is_ok());
    }

    #[test]
    fn flow_with_unknown_lib_key() {
        let flow_description = "\
        flow = 'use-library-function'

        [[process]]
        alias = 'print'
        lib = 'lib://fake/stdio/stdout.toml'
    ";

        let toml = TomlDeserializer::<Process>::new();
        assert!(toml.deserialize(flow_description, None).is_err());
    }

    #[test]
    fn flow_with_function_without_source() {
        let flow_description = "\
        flow = 'use-library-function'

        [[process]]
        alias = 'print'
    ";

        let toml = TomlDeserializer::<Process>::new();
        assert!(toml.deserialize(flow_description, None).is_err());
    }

    #[test]
    fn load_fails_if_no_alias() {
        let flow_description = "\
        [[process]]
        source = 'lib://flowstdlib/stdio/stdout.toml'
    ";

        let toml = TomlDeserializer::<Process>::new();
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

        let toml = TomlDeserializer::<Process>::new();
        assert!(toml.deserialize(function_definition, None).is_ok());
    }

    #[test]
    fn function_lacks_name() {
        let function_definition = "\
implementation = 'stdout.rs'
[[input]]
name = 'stdout'
type = 'String'";

        let toml = TomlDeserializer::<Process>::new();
        assert!(toml.deserialize(function_definition, None).is_err());
    }

    #[test]
    fn function_lacks_implementation() {
        let function_definition = "\
function = 'stdout'
[[input]]
name = 'stdout'
type = 'String'";

        let toml = TomlDeserializer::<Process>::new();
        assert!(toml.deserialize(function_definition, None).is_err());
    }
}

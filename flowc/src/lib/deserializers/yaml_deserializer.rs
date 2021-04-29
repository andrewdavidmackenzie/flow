use url::Url;

use crate::compiler::loader::Deserializer;
use crate::errors::*;
use crate::model::process::Process;

pub struct FlowYamlLoader;

// NOTE: Indexes are one-based
impl Deserializer for FlowYamlLoader {
    fn deserialize(&self, contents: &str, url: Option<&Url>) -> Result<Process> {
        serde_yaml::from_str(contents).chain_err(|| {
            format!(
                "Error deserializing Yaml from: '{}'",
                url.map_or("URL was None".to_owned(), |u| u.to_string())
            )
        })
    }

    fn name(&self) -> &'static str {
        "Yaml"
    }
}

#[cfg(test)]
mod test {
    use serde_yaml::Error;

    use flowcore::manifest::MetaData;

    use crate::compiler::loader::Deserializer;
    use crate::model::process::Process::FlowProcess;

    use super::FlowYamlLoader;

    #[test]
    fn invalid_yaml() {
        let deserializer = FlowYamlLoader {};

        if deserializer.deserialize("{}", None).is_ok() {
            panic!("Should not have parsed correctly as is invalid JSON");
        };
    }

    #[test]
    fn flow() {
        let flow_with_name = "
flow: 'hello-world-simple-toml'
";

        let yaml = FlowYamlLoader {};
        match yaml.deserialize(&flow_with_name.replace("'", "\""), None) {
            Ok(FlowProcess(flow)) => {
                assert_eq!(flow.name.to_string(), "hello-world-simple-toml".to_string())
            }
            Ok(_) => panic!("Deserialization didn't detect a flow"),
            Err(e) => panic!("Deserialization error: {:?}", e),
        }
    }

    #[test]
    fn metadata() {
        let metadata = "\
name: \"me\"
version: \"1.1.1\"
description: \"ok\"
authors: [\"Andrew <andrew@foo.com>\"]
    ";

        let result: Result<MetaData, Error> = serde_yaml::from_str(metadata);
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
    fn flow_with_partial_metadata() {
        let flow_description = "
flow: hello-world-simple-toml

metadata:
  version: '1.1.1'
  authors: ['unknown <unknown@unknown.com>']
";

        let yaml = FlowYamlLoader {};
        match yaml.deserialize(&flow_description.replace("'", "\""), None) {
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
}

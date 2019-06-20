use compiler::loader::{Deserializer, DeserializeError};
use model::process::Process;

pub struct FlowYamlLoader;

// NOTE: Indexes are one-based
impl Deserializer for FlowYamlLoader {
    fn deserialize(&self, contents: &str, url: Option<&str>) -> Result<Process, DeserializeError> {
        serde_yaml::from_str(contents).map_err(|e| {
            let line_col = match e.location() {
                Some(location) => Some((location.line(), location.column())),
                _ => None
            };
            DeserializeError::new("Yaml deserialization error", line_col, url)
        })
    }

    fn name(&self) -> &'static str { "Yaml" }
}


#[cfg(test)]
mod test {
    use compiler::loader::Deserializer;

    use super::FlowYamlLoader;

    #[test]
    fn invalid_yaml() {
        let deserializer = FlowYamlLoader {};

        match deserializer.deserialize("{}", None) {
            Ok(_) => assert!(false, "Should not have parsed correctly as is invalid JSON"),
            Err(e) => assert_eq!(None, e.line_col(), "Should produce syntax error at (1,1)")
        };
    }

    #[test]
    fn simple_context_loads() {
        let flow_description = "
flow: hello-world-simple-toml

version: 0.0.0
author_name: unknown
author_email: unknown@unknown.com
";

        let yaml = FlowYamlLoader {};
        let flow = yaml.deserialize(&flow_description.replace("'", "\""), None);
        println!("{:?}", flow);
        assert!(flow.is_ok());
    }
}
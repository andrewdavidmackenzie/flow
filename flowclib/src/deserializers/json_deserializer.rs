extern crate serde_json;

use std::error::Error;

use compiler::loader::{DeserializeError, Deserializer};
use model::process::Process;

pub struct FlowJsonLoader;

// NOTE: Indexes are one-based
impl Deserializer for FlowJsonLoader {
    fn deserialize(&self, contents: &str, url: Option<&str>) -> Result<Process, DeserializeError> {
        serde_json::from_str(contents).map_err(|e| {
            DeserializeError::new(e.description(), Some((e.line(), e.column())), url)
        })
    }

    fn name(&self) -> &'static str { "Json" }
}

#[cfg(test)]
mod test {
    use compiler::loader::Deserializer;

    use super::FlowJsonLoader;

    #[test]
    fn invalid_json() {
        let deserializer = FlowJsonLoader {};

        match deserializer.deserialize("=", None) {
            Ok(_) => assert!(false, "Should not have parsed correctly as is invalid JSON"),
            Err(e) => assert_eq!(Some((1, 1)), e.line_col(), "Should produce syntax error at (1,1)")
        };
    }


    #[test]
    fn simplest_context_loads() {
        let flow_description = "{
    'flow': 'hello-world-simple-toml',
    'process': [
        {
            'alias': 'print',
            'source': 'lib://runtime/stdio/stdout.toml',
            'input': {
                'default': {
                    'once': 'hello'
                }
            }
        }
    ]
}";
        let toml = FlowJsonLoader {};
        let flow = toml.deserialize(&flow_description.replace("'", "\""), None);
        println!("{:?}", flow);
        assert!(flow.is_ok());
    }

    #[test]
    fn simple_context_loads() {
        let flow_description = "{
    'flow': 'hello-world-simple-toml',
    'process': [
        {
            'alias': 'message',
            'source': 'lib://flowstdlib/data/buffer.toml',
            'input': {
                'default': {
                    'once': 'hello'
                }
            }
        },
        {
            'alias': 'print',
            'source': 'lib://runtime/stdio/stdout.toml'
        }
    ],
    'connection': [
        {\
            'from': 'process/message',
            'to': 'process/print'
        }
    ]
}";
        let toml = FlowJsonLoader {};
        let flow = toml.deserialize(&flow_description.replace("'", "\""), None);
        println!("{:?}", flow);
        assert!(flow.is_ok());
    }

    #[test]
    fn invalid_context_fails() {
        let flow_description = "{
    'flow': 'hello-world-simple-toml',
    'process': [
        {
            'alias': 'message',
            'source': 'lib://flowstdlib/data/buffer.toml',
            'input': {
                'default': {
                    'once': 'hello'
                }
            }
        },
        {
            'alias': 'print',
            'source': 'lib://runtime/stdio/stdout.toml'
        }
    ],
    'connection': [
        {\
            'from': 'process/message'
        }
    ]
}";
        let toml = FlowJsonLoader {};
        let flow = toml.deserialize(&flow_description.replace("'", "\""), None);
        assert!(flow.is_err());
    }
}
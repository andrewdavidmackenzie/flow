extern crate serde_json;

use crate::compiler::loader::Deserializer;
use crate::errors::*;
use crate::model::process::Process;

pub struct FlowJsonLoader;

// NOTE: Indexes are one-based
impl Deserializer for FlowJsonLoader {
    fn deserialize(&self, contents: &str) -> Result<Process> {
        serde_json::from_str(contents).chain_err(|| "Error deserializing Json".to_string())
    }

    fn name(&self) -> &'static str { "Json" }
}

#[cfg(test)]
mod test {
    use crate::compiler::loader::Deserializer;

    use super::FlowJsonLoader;

    #[test]
    fn invalid_json() {
        let deserializer = FlowJsonLoader {};

        match deserializer.deserialize("=") {
            Ok(_) => assert!(false, "Should not have parsed correctly as is invalid JSON"),
            Err(_) => assert!(true, "Should produce syntax error at (1,1)")
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
        let flow = toml.deserialize(&flow_description.replace("'", "\""));
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
        let flow = toml.deserialize(&flow_description.replace("'", "\""));
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
        let flow = toml.deserialize(&flow_description.replace("'", "\""));
        assert!(flow.is_err());
    }
}
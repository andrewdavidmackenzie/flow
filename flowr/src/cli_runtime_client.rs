use std::env;
use std::fs::File;
use std::io;
use std::io::prelude::*;

use flowruntime::runtime_client::{Command, Response, RuntimeClient};

#[derive(Debug)]
pub struct CLIRuntimeClient {}

/// The name of the environment variables used to pass command line arguments to the function
/// used to get them.
pub const FLOW_ARGS_NAME: &str = "FLOW_ARGS";

impl RuntimeClient for CLIRuntimeClient {
    fn init(&self) {}

    // This function is called by the runtime_function to send a commanmd to the runtime_client
    // so here in the runtime_client, it's more like "process_command"
    fn send_command(&self, command: Command) -> Response {
        match command {
            Command::Stdout(contents) => {
                println!("{}", contents);
                Response::Ack
            }
            Command::Stderr(contents) => {
                eprintln!("{}", contents);
                Response::Ack
            }
            Command::Stdin => {
                let mut buffer = String::new();
                let stdin = io::stdin();
                let mut handle = stdin.lock();
                if let Ok(size) = handle.read_to_string(&mut buffer) {
                    if size > 0 {
                        return Response::Stdin(buffer.trim().to_string());
                    }
                }
                Response::Error("Could not read Stdin".into())
            }
            Command::Readline => {
                let mut input = String::new();
                match io::stdin().read_line(&mut input) {
                    Ok(n) if n > 0 => Response::Readline(input.trim().to_string()),
                    _ => Response::Error("Could not read Readline".into())
                }
            }
            Command::Args => {
                let args = env::var(FLOW_ARGS_NAME).unwrap();
                env::remove_var(FLOW_ARGS_NAME); // so another invocation later won't use it by mistake
                let flow_args: Vec<String> = args.split(' ').map(|s| s.to_string()).collect();
                Response::Args(flow_args)
            }
            Command::Write(filename, bytes) => {
                let mut file = File::create(filename).unwrap();
                file.write_all(bytes.as_slice()).unwrap();
                Response::Ack
            }
        }
    }
}

#[cfg(test)]
mod test {
    use std::env;

    use flowruntime::runtime_client::{Command, Response, RuntimeClient};

    use super::CLIRuntimeClient;
    use super::FLOW_ARGS_NAME;

    #[test]
    fn test_arg_passing() {
        env::set_var(FLOW_ARGS_NAME, "test");

        let client = CLIRuntimeClient {};

        match client.send_command(Command::Args) {
            Response::Args(args) => assert_eq!(vec!("test".to_string()), args),
            _ => panic!("Didn't get Args response as expected")
        }
    }

    #[test]
    fn test_file_writing() {
        let temp = tempdir::TempDir::new("flow").unwrap().into_path();
        let file = temp.join("test");

        let client = CLIRuntimeClient {};

        if client.send_command(Command::Write(file.to_str().unwrap().to_string(), b"Hello".to_vec()))
            != Response::Ack {
            panic!("Didn't get Write response as expected")
        }
    }

    #[test]
    fn test_stdout() {
        let client = CLIRuntimeClient {};
        if client.send_command(Command::Stdout("Hello".into())) != Response::Ack {
            panic!("Didn't get Stdout response as expected")
        }
    }

    #[test]
    fn test_stderr() {
        let client = CLIRuntimeClient {};
        if client.send_command(Command::Stderr("Hello".into())) != Response::Ack {
            panic!("Didn't get Stderr response as expected")
        }
    }
}
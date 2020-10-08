use std::collections::HashSet;
use std::env;
use std::fs::File;
use std::io;
use std::io::prelude::*;

use log::debug;

use flowrlib::runtime_client::{Command, Response, RuntimeClient};

#[derive(Debug, Clone)]
pub struct CLIRuntimeClient {
    pub open_files: HashSet<String>
}

/// The name of the environment variables used to pass command line arguments to the function
/// used to get them.
pub const FLOW_ARGS_NAME: &str = "FLOW_ARGS";

impl CLIRuntimeClient {
    pub fn new() -> Self {
        CLIRuntimeClient{
            open_files: HashSet::new()
        }
    }
}

impl RuntimeClient for CLIRuntimeClient {
    fn flow_start(&mut self) {
        debug!("===========================    Starting flow execution =============================");
    }

    // This function is called by the runtime_function to send a commanmd to the runtime_client
    // so here in the runtime_client, it's more like "process_command"
    fn send_command(&mut self, command: Command) -> Response {
        match command {
            Command::EOF => Response::Ack,
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
                    } else {
                        return Response::EOF;
                    }
                }
                Response::Error("Could not read Stdin".into())
            }
            Command::Readline => {
                let mut input = String::new();
                match io::stdin().read_line(&mut input) {
                    Ok(n) if n > 0 => Response::Readline(input.trim().to_string()),
                    Ok(n) if n == 0 => Response::EOF,
                    _ => Response::Error("Could not read Readline".into())
                }
            }
            Command::Write(filename, bytes) => {
                self.open_files.insert(filename.clone());
                let mut file = File::create(filename).unwrap();
                file.write_all(bytes.as_slice()).unwrap();
                Response::Ack
            }
            Command::Args => {
                let args = env::var(FLOW_ARGS_NAME).unwrap();
                env::remove_var(FLOW_ARGS_NAME); // so another invocation later won't use it by mistake
                let flow_args: Vec<String> = args.split(' ').map(|s| s.to_string()).collect();
                Response::Args(flow_args)
            }
        }
    }

    fn flow_end(&mut self) {
        debug!("=========================== Flow execution ended ======================================");

        // TODO close open files
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashSet;
    use std::env;

    use flowrlib::runtime_client::{Command, Response, RuntimeClient};

    use super::CLIRuntimeClient;
    use super::FLOW_ARGS_NAME;

    #[test]
    fn test_arg_passing() {
        env::set_var(FLOW_ARGS_NAME, "test");

        let mut client = CLIRuntimeClient { open_files: HashSet::new() };

        match client.send_command(Command::Args) {
            Response::Args(args) => assert_eq!(vec!("test".to_string()), args),
            _ => panic!("Didn't get Args response as expected")
        }
    }

    #[test]
    fn test_file_writing() {
        let temp = tempdir::TempDir::new("flow").unwrap().into_path();
        let file = temp.join("test");

        let mut client = CLIRuntimeClient::new();

        if client.send_command(Command::Write(file.to_str().unwrap().to_string(), b"Hello".to_vec()))
            != Response::Ack {
            panic!("Didn't get Write response as expected")
        }
    }

    #[test]
    fn test_stdout() {
        let mut client = CLIRuntimeClient::new();
        if client.send_command(Command::Stdout("Hello".into())) != Response::Ack {
            panic!("Didn't get Stdout response as expected")
        }
    }

    #[test]
    fn test_stderr() {
        let mut client = CLIRuntimeClient::new();
        if client.send_command(Command::Stderr("Hello".into())) != Response::Ack {
            panic!("Didn't get Stderr response as expected")
        }
    }
}